---
id: T31
title: "Integration and hand-back"
status: done
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
   one was parked as an Open point rather than handed back and repaired; whether any page
   still describes the prototype branches, the bootstrap binary that printed one line, or
   a runtime that is not chosen; whether a page cites 0014 as current; whether the
   open-question list matches the module's `open question` blocks one for one; whether a
   disposition in step 2 cites a gate that does not run the item; whether the fresh-clone
   quote came from the commit it names; whether a search that must print nothing was run
   without its positive control. Two reviewer subagents with fresh contexts run in
   parallel, one lens each, `conformance` and `breakage`, each running `codex exec -m
   gpt-6-astra -s read-only -C <worktree> -o ai_tmp/review/T31-1/findings-<lens>.md
   "<brief>" < /dev/null` (§14's command; `-o` writes only the reviewer's last message,
   and a read-only reviewer writes no file of its own); separate triage subagents check
   every finding against the tree and write `ai_tmp/review/T31-1/verdicts.md`, each
   finding valid with its fix or invalid with the reason; one writer applies the valid
   fixes one at a time, only within Files touched, and hands back any fix outside it; if
   a high-severity finding was valid, a second round reviews the fix diff and writes
   beside the first with a `-2` suffix; two rounds at most. A fix after the clone makes
   step 7's quotes stale: rerun step 7. Then `just check` (`project-check` skill; a
   failing recipe goes to `fix-quality`), set `status: done`, and commit B with the
   hand-back notes, each quoted output headed by the commit it ran at.
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

### Baseline

At `0927fe5` (T22 to T30 merged into the wave branch), `just check` on the untouched
worktree (exit 0):

```text
All checks passed and the worktree is unchanged.
```

Commit A is `5faacc0` (the sweep); commit A2 is `40ff3e7` (the T31-1 and `code-review`
fixes); commit B carries this section and `status:`.

### Wave overrides

- Worked on the wave branch `mvp-inimal-but-sufficient` from `0927fe5`, not on
  `ticket/t31-mvp-integration` from `main`. Wherever the ticket says
  `git diff main...HEAD`, the diff was taken against the start commit plus the
  uncommitted changes.
- `git fetch origin`, merging `origin/main`, push and pull request: not run: one pull
  request at T31. That pull request is the wave's, and the lead opens it (step 9).
- Step 7 clones the wave branch, `mvp-inimal-but-sufficient`, not
  `ticket/t31-mvp-integration`.
- Step 9 stops before `git push`: the lead pushes, opens the single pull request
  (authorised) and reads its CI run.
- Step 10 drafts the two `CONVENTIONS.md` sentences below and stops; the edit is a
  separate `main` pull request.
- Beside `review-change` at step 5, the lead asked for the `code-review` skill over the
  wave. This agent invoked it twice; each run reported to the lead, who forwarded its
  findings, and this agent dispositioned them (below).
- The §14 reviews, triage and the fix writer were run by the wave lead; this agent
  stopped uncommitted at the checkpoint, read the fix diff and reran its checks.

### Deviations from Files touched and the steps

Files touched widened, each approved by the lead:

- `docs/tutorials/first-change.md` §2 to §5, beyond "lines 60 to 62": the tutorial no
  longer compiles as written. It targeted the old `run(out) -> io::Result<()>`; the
  version line is now built in `crates/pachinko/src/lib.rs`'s private `fn version`
  (step 2's D-T24-5, so it is not an Open point). The exercise is quoted under step 4.
- `docs/explanation/layering.md:20-31` (`review-change` class 3, item 1) and `:72-76`
  (T31-1 C2).
- `docs/how-to/test-and-debug.md:98`, "The ack and release lines arrive with T25"
  (`review-change` class 3, item 2).
- `crates/pachinko/Cargo.toml:7`, the package description (`just lock-check` and
  `just toml-check` exit 0).
- `rustqual.toml`'s comment and `docs/reference/configuration.md:101` (T31-1 C2).
- `crates/pachinko/src/serve.rs` and `crates/pachinko/src/actor.rs`: the code changes for
  `code-review` run 2's findings 2 and 10.

Edits within own files beyond the line ranges the rows name: `AGENTS.md:9-10`;
`repository-map.md`'s tree block and the `libpachinko/tests` row; `purpose-and-scope.md`'s
intro; `docs/README.md`'s Understand bullet; `CHANGELOG.md:62`; `terminology.md`'s
Library and Effect trait rows; `crates/libpachinko/README.md`'s `Queue` sentence;
`api.md:36`; `security-model.md`'s "What there is to protect" bullet.

Rows with nothing to do:

- `docs/reference/testing.md:83-84`: a lane had already replaced the placeholder
  sentence.
- `crates/pachinko/README.md`: T24 replaced the file; it describes tokio, axum and 0019.
- `docs/project/terminology.md:49`: the 0014 half was already gone (T22); the "minimal
  binary" half was fixed in A.
- Decisions 0014 and 0019: the cross-references were already correct (0014 line 11,
  "Decision 0019 supersedes this record"; 0019 line 11, "This record supersedes
  decision 0014").
- `tickets/README.md`'s "Review of the ticket set" bullet is not "Pending": line 134
  reads "were reviewed on". Step 6's condition did not fire, and the bullet was left
  alone.

The step 7 clone is not at `ai_tmp/fresh`. A clone nested in the worktree makes cargo
resolve `fuzz/Cargo.toml` against the outer worktree's workspace ("current package
believes it's in a workspace"), so `just initialize` failed at `sync`. The clone went to
the session's scratch directory instead (its path is in the quotes below).

Ticket commands adjusted, each quoted below:

- Routes: `rg -n '\.route\(' crates/pachinko/src/http.rs` prints 13. The 13th is the
  test router at `http.rs:749`, under `#[cfg(test)]`, so the routes are counted before
  `#[cfg(test)]`: 12.
- Open questions: `rg -c 'open question'` prints 13, because five are comment mentions;
  `rg -c '^open question'` prints 8, the blocks at `pachinko.allium:886-893`.
- `NeverRefusedForCapacity`: the ledger assigns it to T27, which tagged it in
  `crates/pachinko` as `// guarantee:`, so the search runs over `crates`, not
  `crates/libpachinko`.

### Step 2: the hand-backs reconciled

The full reconciliation, with every quote, source and tree check, is
`ai_tmp/wave/T31/step2-reconcile.md`, read-only at `0927fe5`.

| Disposition | Count |
| --- | --- |
| (a) fix in T31 | 14 (A-1 to A-14) |
| (b) closed by evidence another ticket quoted | 27 (B-1 to B-27), plus D-T30-9 and L-1 |
| (c) missing MVP obligation | 0 |
| (d) open with an owner | 50 |

The (a) rows and their outcome:

- A-1 `README.md:10-12`, the domain undefined and the placeholder module: fixed in A.
- A-2 `AGENTS.md:5` and `:13`: fixed in A; `just check-agents` green.
- A-3 `docs/README.md:15-21`, the intro and the placeholder paragraph: fixed in A.
- A-4 `docs/project/purpose-and-scope.md:40-47`, "Until T24 builds it": fixed in A.
- A-5 `docs/project/terminology.md:49`, "a minimal binary": fixed in A; the 0014 half
  had nothing to do.
- A-6 `docs/project/repository-map.md:89` and `:95`, the `version()`-only library and
  "declares no behaviour": fixed in A.
- A-7 `docs/tutorials/first-change.md:61`: fixed in A, with §2 to §5 (deviations).
- A-8 `docs/explanation/architecture.md:252`, T24's "5-second grace": fixed in A, with
  readiness and `--shutdown-grace-ms` and a pointer to the HTTP API's shutdown section.
- A-9 `CHANGELOG.md:54` and `:57-58`, "not yet implemented" and "awaits its
  implementation": fixed in A, and `:62`'s placeholder module.
- A-10 `docs/reference/configuration.md:196-197`, "declares nothing": fixed in A.
- A-11 `tickets/README.md`, T22, T23, T25, T26 and T28 `todo`: fixed in A; T31's own row
  goes with its `status:` at commit B.
- A-12 `crates/pachinko/README.md`: nothing to do (T24 replaced it).
- A-13 `crates/libpachinko/README.md:7-8`, the `Queue` sentence leaving out the lease
  actions and `tick`: completed in A.
- A-14 `docs/explanation/security-model.md`: "What is deferred" was already renamed by a
  lane; the Related pages' 0014 link was dropped in A.

The (b) rows close by the ticket and section each names: B-1 to B-27 in
`step2-reconcile.md`; no green gate is cited for an item it does not run (`plan-spec`,
`test-loadgen`, `image`, `bench` and `loadgen` are outside `check`). The wave-override
"not run" group (fetch, merge, push and pull request in T22 to T30, `just initialize` in
T22, T23, T24 and T26) closes with this ticket's step 7 and the lead's step 9. Every (d)
row not already in Open points is carried there with its owner. No hand-back reported a
missing MVP obligation; the near-misses are N-1 (`NeverRefusedForCapacity`'s location),
N-2 (T26-1's "192 obligations", a transcription slip; the count is 190) and N-3 (the
open-question count), each settled in step 3 or 4.

### Step 3: the ownership sweep

At `0927fe5` with uncommitted changes (the sweep that became A). Each of the 61 rules,
invariants and guarantees in libpachinko's columns prints at least one `// obligation:`
tag under `crates/libpachinko` (`ai_tmp/wave/T31/step3-sweep.txt`, one line per name);
`NeverRefusedForCapacity` prints none there, as N-1 expected. The scenario, route, flag
and code lines, with their positive controls (`ai_tmp/wave/T31/step3-verif.txt`, the
flag-arm listing trimmed):

```text
== HEAD 0927fe5
$ rg -c scenario counts
crates/libpachinko/tests/queue/expiry.rs:19
crates/libpachinko/tests/queue/lease.rs:12
crates/libpachinko/tests/queue/publish.rs:12
crates/libpachinko/src/config.rs:1
crates/libpachinko/tests/queue/identity.rs:7
crates/libpachinko/tests/queue/pull.rs:11
crates/libpachinko/tests/queue/settle.rs:11
crates/libpachinko/tests/queue/decay.rs:3
exit 0
$ diff
exit 0
$ dup
exit 0
$ routes
80:        .route("/healthz", get(healthz))
81:        .route("/readyz", get(readyz))
95:        .route("/publishers/{publisher_id}", post(publisher_joins))
96:        .route("/consumers/{consumer_id}", post(consumer_joins))
97:        .route("/pockets/{pocket_key}/records/{record_key}", put(publish))
98:        .route(
102:        .route("/consumers/{consumer_id}/pull", post(pull))
103:        .route("/consumers/{consumer_id}/lease", get(lease))
104:        .route("/leases/{lease_id}/ack", post(leases::ack))
105:        .route("/leases/{lease_id}/nack", post(leases::nack))
106:        .route("/leases/{lease_id}/renew", post(leases::renew))
107:        .route("/leases/{lease_id}/release", post(leases::release))
749:            Router::new().route("/never", get(std::future::pending::<()>)),
13
$ flags
13
exit 0
$ codes: arms in code()
14
wildcard-exit 1
$ positive control: diff against S001..S077 (must print and exit 1)
76a77
> scenario: S077
exit 1
$ positive control: uniq -d with S001 doubled (must print)
scenario: S001
exit 0
$ positive control: the pattern takes the whole token (S0761 must fail diff)
77d76
< scenario: S0761
exit 1
$ plan-spec obligation count
190
$ catalog rows 76
ids exactly S001..S076: True
```

The flag arms are `config.rs:267-288`, 13 `"--name" =>` lines. The catalog check's one
"unparsed" row is S005, whose owner is the `compile_fail` doctest on `Config`
(`config.rs:45-46`, `// scenario: S005`), which names no `file`, `fn` pair. The 190 is
`obligations[]` in `just plan-spec pachinko`'s output (`ai_tmp/wave/T31/plan-spec.txt`),
as T22, T25 and T28 quoted.

The adjusted `NeverRefusedForCapacity` search, at `40ff3e7`:

```text
$ rg -n 'NeverRefusedForCapacity' crates; echo "exit $?"
crates/pachinko/src/actor.rs:833:    // guarantee: NeverRefusedForCapacity
crates/pachinko/tests/batch_publish.rs:259:// guarantee: NeverRefusedForCapacity
exit 0
```

No name was missing, so no lane reopened. Steps 3 to 5 ran before A2, which changed
`serve.rs` and `actor.rs`; at A2 the fresh clone's `just check` (step 7) is the gate.

### Step 4: the handbook sweep

At `0927fe5` with uncommitted changes (the sweep that became A), from
`ai_tmp/wave/T31/step4-verif.txt`. The positive control is `git grep` at `611a86e`, with
no checkout; long lines are cut at `...`:

```text
== positive control at 611a86e (git grep, no checkout)
611a86e:AGENTS.md:5:The workspace holds a library, `libpachinko`, and a service, `pachinko`, built around it. Their domain is not defined yet. ...
611a86e:AGENTS.md:13:- `docs/specs/` holds the Allium specifications; today it holds one placeholder module.
611a86e:README.md:10:The domain is not defined yet. The Allium specifications under `docs/specs/` decide
611a86e:README.md:11:behaviour; today they contain one placeholder module with no declarations. The library
611a86e:crates/libpachinko/src/lib.rs:4://! The domain is not defined yet, so this crate holds one item. When it grows, it
611a86e:crates/pachinko/README.md:8:The domain is not defined yet. The Allium specifications under `docs/specs/` will
611a86e:crates/pachinko/README.md:10:of the library's effect traits. No runtime, transport or HTTP stack has been chosen.
611a86e:docs/explanation/architecture.md:160:The service has no async runtime, transport or HTTP stack, and none is chosen. Choosing
611a86e:docs/project/purpose-and-scope.md:44:  no async runtime, transport or HTTP stack. Choosing them before the specifications say
611a86e:docs/project/repository-map.md:89:| `docs/specs/` | The Allium specifications. ... which declares no behaviour. |
611a86e:docs/project/terminology.md:49:| Service | `pachinko`, under `crates/pachinko/`: a minimal binary built around the library, ...
611a86e:docs/reference/configuration.md:156:declares nothing, so the code mirrors no figure; see
611a86e:docs/reference/testing.md:84:states no behaviour, so `just plan-spec pachinko` has nothing to ask for.
611a86e:docs/tutorials/first-change.md:61:entity, rule, surface or invariant: the domain is not defined yet, and the first change
exit 0
== working tree on top of 0927fe5
exit 1
== rg -n 0014 docs
docs/manifest.yml:51:    {"path": "decisions/0014-service-runtime-deferred.md", "title": "Decision 0014: ...", ...},
docs/decisions/0008-std-library.md:32:([decision 0014](0014-service-runtime-deferred.md)). `rust-toolchain.toml` names no
docs/decisions/0008-std-library.md:138:- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
docs/decisions/0001-library-and-service.md:40:specifications ([decision 0014](0014-service-runtime-deferred.md)).
docs/decisions/0001-library-and-service.md:92:- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
docs/decisions/0014-service-runtime-deferred.md:2:title: "Decision 0014: The service runtime waits for the specifications"
docs/decisions/0014-service-runtime-deferred.md:9:# Decision 0014: The service runtime waits for the specifications
docs/decisions/README.md:37:| [0014](0014-service-runtime-deferred.md) | The service runtime waits for the specifications |
docs/decisions/README.md:45:0004 it replaces and the ones that stand. Decision 0014 is superseded by decision 0019,
docs/decisions/README.md:46:which replaces its deferral with the runtime the prototypes chose; 0014's consequences
docs/decisions/README.md:57:reference names. Three entries were new at the foundation: 0001, 0008 and 0014. Nothing
docs/decisions/0015-container-image.md:16:[decision 0014](0014-service-runtime-deferred.md) leaves its eventual surface to the
docs/decisions/0015-container-image.md:54:- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
docs/decisions/0019-service-runtime-tokio-and-http.md:11:This record supersedes [decision 0014](0014-service-runtime-deferred.md), which deferred
docs/decisions/0019-service-runtime-tokio-and-http.md:16:Decision 0014 chose no async runtime, transport or HTTP stack and named its trigger: the
docs/decisions/0019-service-runtime-tokio-and-http.md:19:that trigger. The spike 0014 called for then ran as three prototypes, each building the
docs/decisions/0019-service-runtime-tokio-and-http.md:102:**This record.** It is one fresh record, 0019, that supersedes 0014 and cites PR 38 at
docs/decisions/0019-service-runtime-tokio-and-http.md:297:- [Decision 0014: The service runtime waits for the specifications](0014-service-runtime-deferred.md)
== rg -c '^open question' (ticket's rg -c 'open question' prints:)
13
8
== routes before #[cfg(test)]
12
```

The working-tree sweep prints nothing (exit 1). `rg -n '0014' docs` prints only the
survivors step 4 names: 0014 itself, the decisions index's row and foundation sentence,
0019's citations, the manifest entry, and 0001, 0008 and 0015 citing it historically.
`docs/how-to/work-with-the-specs.md` lists the eight questions in module numbering.

The tutorial exercise: with `first-change.md` §2 to §5's edit applied as written, then
reverted. The three expected failures (`ai_tmp/wave/T31/tut-nextest.log` and
`tut-doc.log`, trimmed; the logs carry no commit line, and they ran on the uncommitted
sweep before A):

```text
     Summary [   0.632s] 236 tests run: 234 passed, 2 failed, 0 skipped
        FAIL [   0.006s] (183/236) pachinko tests::run_writes_the_name_and_the_library_version
        FAIL [   0.302s] (227/236) pachinko::binary the_binary_prints_its_line_and_exits_zero
error: test run failed
exit 100
failures:
    crates/pachinko/src/lib.rs - run (line 47)
test result: FAILED. 26 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
error: doctest failed, to rerun pass `-p pachinko --doc`
error: recipe `test-doc` failed on line 144 with exit code 101
exit 101
```

### Step 5: `review-change` over the wave

Over `git diff 611a86e` at `0927fe5` plus the uncommitted sweep
(`ai_tmp/wave/T31/review-change.md`). The ten invariants hold; `just effects` exit 0
("libpachinko names none of 8 banned crates and uses none of 121 banned std APIs";
"pachinko trips none"); coverage 99.01% of lines. Class 2, a missing MVP obligation:
none.

- Class 1, all low, applied in A: `architecture.md` restated the shutdown contract the
  HTTP API page owns; `api.md:37` named only `run` and `exit_code` as items with
  examples; `security-model.md:133-134`'s "the prototype's resident memory" now names
  PR 38's prototype.
- Class 3: `layering.md:20-31` (medium) and `test-and-debug.md:98` (low) were fixed in
  A under the widened Files touched; `counting.rs`'s `thread_local!` (T26) and the flag
  defaults restated on five pages (T27, T29) are in Open points.

### `code-review` beside step 5

Run 1, level medium, hit the wrong target: commit `611a86e` against its first parent, so
it reviewed T19's specification, not the wave (`ai_tmp/wave/T31/code-review-run1.md`).
Dispositions:

- 1, 2, 4 and 5 are questions about the specification's behaviour, so product decisions:
  Open points for the maintainer as proposed open questions. The specification was not
  edited.
- 3: already fixed by T22 (branch A, the `lease_ttl = 0` caveat).
- 6, "consumer" used for the library's caller: still stale at `5faacc0` in
  `architecture.md:13`, `:211` and `:217`, `api.md:68` and `:70`,
  `configuration.md:135`, `security-model.md:145` and `testing.md:31`; fixed in A2.
- 7, the domain-undefined prose: fixed in A.
- 8, 9 and 10, low: Open points.

Run 2, level medium, over the branch against `main` with the uncommitted changes,
`crates/` included (`ai_tmp/wave/T31/code-review-run2.md`). Each finding was verified
against the tree:

- 1, `http-api.md:98`, lost-batch recovery: verified (`PullOutcome::Batch` carries the
  lease id, a pull within the lease returns only newly held records, and the limit
  clamps to 0); the page now gives the recovery. Fixed in A2.
- 2, `serve.rs`, `--shutdown-grace-ms 0`: reproduced. The binary exited 1 on 1 of 5
  SIGTERMs after start ("shutdown took longer than 0ms"), and a unit test was red
  (`Err(Task(..0ms..))` against `Ok`). Fixed test-first in A2 (lead-approved `serve.rs`
  deviation): a zero grace cuts at once and joins the tasks unbounded, aborting none, so
  it is no overrun; a grace above zero is unchanged.
  `a_zero_grace_period_cuts_at_once_and_joins_the_tasks_that_end_on_the_cut` went red,
  then green; `a_stop_with_a_zero_grace_period_ends_serve_with_ok` passed before the
  fix too and stays as the `serve`-level guard. After the fix, 30 of 30 SIGTERMs after
  the ready line exited 0. `configuration.md:47` and the HTTP API's Shutdown section say
  so.
- 3, the lease-id exhaustion guard and the redraw loop: Open point (an API change).
- 4, a forward step of the wall clock lapses every lease: Open point; the trade-off is
  decided in 0019 (lines 84 to 88 and 287).
- 5, `loadgen/` returns `String` errors: Open point for the maintainer.
- 6, `http-api.md:64` named `--lease-ttl`: now `--lease-ttl-ms`, fixed in A2; `rg` over
  `docs`, `crates` and `README.md` found no other flag spelled without its unit.
- 7 and 8, `yield_now` per command and a clock read per batch row: Open points (perf).
  Changing either now would invalidate T30's figures.
- 9, `Lifecycle::start`'s fallback tick lane: Open point (latent; no second start
  exists).
- 10, `actor.rs:536` hand-wrote the reply: now uses `answer()`, fixed in A2.

After the fixes, with the content of A2 uncommitted on A: `just fmt-check` exit 0,
`just test` 238 of 238 and the doctests, `just check` green. The fresh clone's quotes
below are the requote at A2.

### Step 6: `tickets/README.md`

T22, T23, T25, T26 and T28 were set to `done` in A; T24, T27, T29 and T30 already read
`done`. Each matches its ticket's `status:`. T31's row goes with its `status:` at B.

### Step 7: the fresh clone

The first run, at A (`5faacc0`), is superseded. The clone at `ai_tmp/fresh` failed (see
deviations); the clone in the scratch directory was green online. A2 then made those
quotes stale (§11), so step 7 was rerun at A2.

At A2, `40ff3e7`, cloned from the local repository into the session's scratch directory.
`just initialize` (`ai_tmp/wave/T31/fresh2-init.log`, trimmed):

```text
== clone HEAD 40ff3e741f485a140a3898c2a6d0d962ffae060f
== 40ff3e7 just initialize (/private/tmp/claude-501/-Users-scutting--supacode-repos-pachinko-mvp-inimal-but-sufficient/edc3394f-8bfd-4b33-b4c0-37cc80e6e50a/scratchpad/fresh)
sh scripts/initialize.sh
...
Installed Git hook at `.git/hooks/pre-commit`
prek prepare-hooks --config .pre-commit-fix.yaml
RUSTUP_AUTO_INSTALL=0 prek run lychee --files README.md
lychee...................................................................Passed

Ready. Next: just check.
Nothing has been formatted, staged, committed, tagged, or pushed.
exit 0
```

`just check` online (`ai_tmp/wave/T31/fresh2-check.log`, 1,159 lines, trimmed to its
recipes' summary lines and its end):

```text
== 40ff3e7 just check
bg-project-check run
...
::notice::Quality score: 100.0% (410 functions analyzed)
::notice::Quality score: 100.0% (346 functions analyzed)
...
effects: libpachinko names none of 8 banned crates and uses none of 121 banned std APIs;
effects: the fixture trips every ban exactly once, and pachinko trips none.
...
TOTAL                                      7199               132    98.17%         603                 5    99.17%        4339                40    99.08%           0                 0         -
...
Validated 46 pages and 47 canonical topics.
...
Validated AGENTS.md, 2 adapters, and 14 skills.
...
        PASS [   0.541s] (238/238) pachinko http::tests::the_api_routes_are_bounded_and_the_probes_answer_beside_them
────────────
     Summary [   0.634s] 238 tests run: 238 passed, 0 skipped
info: no unreferenced snapshots found
info: no snapshots to review

==> just check-clean
bg-project-check clean "$1"
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit 0
```

Under §11's literal macOS profile (`ai_tmp/wave/T31/fresh2-denied-strict.log`, trimmed),
`just check` fails at `coverage`: four of `pachinko`'s in-process HTTP tests cannot bind
127.0.0.1.

```text
== 40ff3e7 profile strict: (version 1)(allow default)(deny network*)
$ curl static.rust-lang.org (must fail)
curl: (6) Could not resolve host: static.rust-lang.org
exit 6
$ curl 1.1.1.1 (must fail)
curl: (7) Failed to connect to 1.1.1.1 port 443 after 0 ms: Couldn't connect to server
exit 7
$ curl static.rust-lang.org without the profile
exit 0
$ curl 1.1.1.1 without the profile
exit 0
$ just check
...
    thread 'http::tests::the_api_routes_are_bounded_and_the_probes_answer_beside_them' (456003195) panicked at crates/pachinko/src/http.rs:643:63:
    called `Result::unwrap()` on an `Err` value: Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }
...
     Summary [   0.175s] 172/238 tests run: 168 passed, 4 failed, 0 skipped
        FAIL [   0.009s] (155/238) pachinko http::tests::a_handler_that_never_completes_answers_408_with_the_body
        FAIL [   0.010s] (162/238) pachinko http::tests::a_streamed_body_over_the_cap_is_413_before_its_content_type_is_checked
        FAIL [   0.008s] (167/238) pachinko http::tests::readyz_is_503_until_serving_204_while_serving_and_503_once_stopping
        FAIL [   0.007s] (168/238) pachinko http::tests::the_api_routes_are_bounded_and_the_probes_answer_beside_them
...
error: recipe `coverage` failed on line 269 with exit code 100
error: recipe `check` failed on line 348 with exit code 100
exit 100
```

The lead's decision: a profile that denies everything but loopback satisfies §11, whose
Linux form keeps `lo` in the namespace. Under it (`ai_tmp/wave/T31/fresh2-denied-lo.log`,
trimmed), both negative controls fail and `just check` is green:

```text
== 40ff3e7 profile lo: (version 1)(allow default)(deny network*)(allow network-bind (local ip "localhost:*"))(allow network-inbound (local ip "localhost:*"))(allow network-outbound (remote ip "localhost:*"))
$ curl static.rust-lang.org (must fail)
curl: (6) Could not resolve host: static.rust-lang.org
exit 6
$ curl 1.1.1.1 (must fail)
curl: (7) Failed to connect to 1.1.1.1 port 443 after 0 ms: Couldn't connect to server
exit 7
$ curl static.rust-lang.org without the profile
exit 0
$ curl 1.1.1.1 without the profile
exit 0
$ just check
...
==> just check-clean
bg-project-check clean "$1"
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit 0
```

A looser profile was tried first and discarded:
`(version 1)(allow default)(deny network*)(allow network* (local ip "localhost:*"))(allow
network* (remote ip "localhost:*"))` let `curl --noproxy '*' https://1.1.1.1` through
(exit 0), so it is no denial. The gap in §11 is in Open points.

### T31-1: the whole diff (step 8)

Harness and model: two reviewer lenses, `conformance` and `breakage`, each
`codex exec` v0.166.0 `-m gpt-6-astra -s read-only` (exit 0), with separate triage and
one writer, run by the wave lead; outputs under `ai_tmp/review/T31-1/`. The breakage
reviewer's sandbox could not run `just check` or the pinned toolchain, so it read the
logs under `ai_tmp/wave/T31/` and the clone at A; it reported no findings. 2 findings,
2 valid, none high, so no second round.

| Finding | Verdict | One-line reason |
| --- | --- | --- |
| C1 | valid, low (re-rated from medium) | `security-model.md:23`, "Nothing is collected or stored", contradicted the in-memory queue the same page describes |
| C2 | valid, low | `layering.md:72-76`, `configuration.md:101` and `rustqual.toml:27` explained the missing pattern rules by "no modules", against the seven Rust modules the page now lists |

Applied: C1 by the lead's writer ("Nothing is persisted or sent elsewhere"). C2 lay
outside Files touched and was handed back; the lead approved it as a deviation and this
agent applied it ("several specification modules"). Handed back: nothing open.

Rerun after the fixes: `just check-docs`, `just metrics` and `just toml-check` exit 0 on
the uncommitted fix; at A2 the fresh clone's `just check` above runs all three
("Validated 46 pages and 47 canonical topics."; "Quality score: 100.0%").

### Final `just check`

On `40ff3e7` with this commit's content uncommitted, which is the tree committed as B
(`git status --short` printed only this file and `tickets/README.md`), exit 0:

```text
All checks passed and the worktree is unchanged.
```

`just check-docs` (`Validated 46 pages and 47 canonical topics.`) and `just check-agents`
(`Validated AGENTS.md, 2 adapters, and 14 skills.`) run inside it.

### Step 9: fetch, push and the pull request

Not run by this agent: `git fetch origin` was not run, and the lead pushes, opens the
pull request and reads its CI run (wave overrides). The CI run URL and the tested merge
commit are the lead's to record. The step 7 clone in the session's scratch directory,
which stands for `ai_tmp/fresh`, was deleted after the quotes above.

### Step 10: the `CONVENTIONS.md` hand-back

Drafted here and not edited on the lane; the edit lands through a separate `main` pull
request.

§0, line 32, replacing "The domain is undefined: no ticket here writes domain code,
domain prose or a behavioural specification.":

> The bootstrap tickets, T00 to T18, wrote no domain code, domain prose or behavioural
> specification; T19 specified the domain in `docs/specs/pachinko.allium`, and the MVP
> wave, T22 to T31, implemented it.

§1, decision 1, after "No ticket names a domain noun.":

> (2026-11-07: T19 defined the domain in `docs/specs/pachinko.allium`, and T22 to T31
> implemented it; the decision governed the foundation tickets, T00 to T18.)

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
- `docs/decisions/0016-release-process.md:63`, "three locks", and
  `docs/decisions/0018-renovate.md:49`, "three places" (D-T30-7): decision records, which
  Non-goals forbid editing here, so T30's acceptance criterion that `rg -n 'three lock'
  docs/` prints no lockfile sentence stays unmet until the maintainer amends both or
  rules them historical. Owner: maintainer.
- §11's macOS denial profile predates tests that bind loopback: `pachinko`'s in-process
  HTTP tests fail under it (step 7). The lead ruled the loopback-only profile satisfies
  §11, as the Linux form keeps `lo`; §11 should say so. Owner: maintainer, through a
  `main` pull request; not a T24 finding.
- Proposed open question (`code-review` run 1, finding 1): a supersede of a held record
  sheds the payload the consumer still holds, so a later nack dead-letters it too and it
  sits in both shed queues. Owner: maintainer.
- Proposed open question (run 1, finding 2): `DropStalePublish` still touches the
  pocket, so a stale publish revives a parked pocket and resets its expiry count. Owner:
  maintainer.
- Proposed open question (run 1, finding 4): `PullWithinLease` extends the lease on an
  empty batch, so a consumer pulling in a loop holds an emptied pocket. Owner:
  maintainer.
- Proposed open question (run 1, finding 5): only a release or a revive resets
  `consecutive_expiries`, so a steadily drained pocket can park after three lapses.
  Owner: maintainer.
- Specification design, low (run 1, findings 8 to 10):
  `docs/explanation/specifications.md` restates the open-question list beside the
  module; `ReviveIdlePocket` and `ReviveParkedPocket` duplicate each other; redundant
  guards give obligations no test can falsify. Owner: maintainer.
- The lease-id exhaustion guard in `actor.rs` stops the service even for a pull that
  needs no new id, and `fresh_lease_id`'s redraw loop spins on a source that repeats an
  active id; the fix is a fallible `LeaseIds::next`, an API change (`code-review` run 2,
  finding 3). Owner: T23 and T24, so the maintainer.
- A forward step of the wall clock lapses every lease at once (run 2, finding 4): the
  trade-off 0019 records at lines 84 to 88 and 287; it waits on the suspend-aware clock
  above. Owner: maintainer.
- `loadgen/` returns `String` errors: whether invariant 10 ("Errors are `thiserror`
  enums") covers the excluded dev crates is open; `fuzz/fuzz_targets` defines no error
  type (run 2, finding 5). Owner: maintainer.
- Throughput, not changed because it would invalidate T30's figures: the owner's
  `yield_now` after every command and a clock read per batch row (run 2, findings 7 and
  8). Owner: maintainer.
- `Lifecycle::start`'s fallback tick lane drops its sender, so a second start would never
  tick; latent, as no second start exists, and documented at `serve.rs:80` (run 2,
  finding 9). Owner: maintainer.
- `crates/libpachinko/tests/queue/counting.rs` logs store calls through a
  `thread_local!`, against the `review-change` skill's rule; its header gives the cause.
  Owner: T26, so the maintainer accepts it or asks for an injectable spy.
- Flag defaults are restated outside `configuration.md`, which owns them, in
  `http-api.md`, `security-model.md`, `troubleshooting.md` and
  `build-the-container-image.md`, so a default change touches five pages; they agree
  today (`review-change` class 3). Owner: T27 and T29, so the maintainer.
- The surviving open questions keep T19's numbers with a gap at eight; this ticket's
  Context adopts "does not compact", and renumbering would be a `spec-change` (D-T22-4).
  Owner: maintainer.
- T21's branch `ticket/t21-pocket-log-records` (0019 cites `94fc9b8`) is local; no gate
  dereferences it, and pushing it needs authorisation (D-T22-5). Owner: maintainer.
- No gate runs doctests with default features: `test` and `test-doc` pass
  `--all-features`, and `features` only builds (D-T23-2). Owner: maintainer.
- The hook gate's `prek run --all-files` reads tracked files only, so a new file's
  EditorConfig fault surfaces only at commit, as in T23 and T30 (D-T23-3). Owner:
  maintainer.
- The lease view carries no lease id, as the spec's `surface Consuming` exposes none; a
  consumer that lost its batch reply recovers the id with a pull of limit 0, and whether
  a route should expose a lease by its id is a product decision (D-T24-1, D-T25-5).
  Owner: maintainer.
- A requester that disconnects between the owner's `is_closed` check and the grant holds
  a lease nobody acts on until it lapses; `http-api.md` documents it (D-T24-2). Owner:
  maintainer.
- The forced arm in each library-error conversion answers 503 `not_ready`; a dedicated
  code is the maintainer's choice (D-T24-3).
- `--listen 0.0.0.0:<port>` opens the surface without a warning; whether it should warn
  is the maintainer's (D-T24-4).
- A zero TTL is admitted and pinned (S006, S007) with no positive floor; whether any TTL
  needs one (D-T26-3). Owner: maintainer.
- `TickReport` carries the lapsed leases in full, if a `debug` line per lease is ever
  wanted; no ticket asks for it (D-T26-5, low). Owner: maintainer.
- With the defaults, a 1,000-row batch of 1 MiB payloads exceeds `--max-body-bytes`; the
  three defaults are the comparables' figures (D-T27-3). Owner: maintainer.
- An empty `records` array is 400 `bad_request`, not `{"outcomes": []}` (D-T27-4).
  Owner: maintainer.
- `tests/batch_publish.rs` and `tests/http.rs` each carry a client helper; folding them
  into `tests/common/mod.rs` is a test refactor this ticket declined (D-T27-5). Owner: a
  later ticket.
- `a_lease_action_body_over_the_cap_is_body_too_large` was not shown to bite separately
  at T27's merge (D-T27-7). Owner: T27's lane, so the maintainer.
- `rule-failure.RequeueDeferredRecord.2` and `rule-failure.RequeueHeldRecord.2` are
  tagged, but no test can observe their rejection; a specification-level question
  (D-T28-1). Owner: maintainer.
- `testing.md` says proptest persists no regression file; whether to commit them instead
  is the maintainer's preference (D-T28-2).
- The invariant walk's domains are small on purpose; a wider walk is a later choice
  (D-T28-3, low). Owner: maintainer.
- Obligations whose only test is the walk are listed in T28's hand-back; whether each
  wants a direct test is for T23, T25 and T26, so the maintainer (D-T28-4).
- Running the walk over `LogRecords` is a one-line instantiation when T21's store lands
  (D-T28-5, low). Owner: a later ticket.
- Whether a service far behind on its command channel should report not ready, beside
  open question 3 (D-T29-1). Owner: maintainer.
- The idle timeout and the handler timeout share one flag (D-T29-2, low). Owner:
  maintainer, on an operator's request.
- The grace period, its second bound, the owner's longest step and `docker stop`'s ten
  seconds are aligned by hand, and the owner's overrun past the second bound is measured
  by no test (D-T29-3). Owner: maintainer.
- A metrics endpoint and a structured log format are planned by no ticket (D-T29-5,
  low). Owner: maintainer.
- A client trickling one byte per bound keeps a connection open until the shutdown's
  cut; `security-model.md` documents it (D-T29-6). Owner: maintainer.
- Docker Desktop on the maintainer's machine hangs removing containers, and clearing the
  `Dead` ones needs a daemon restart (D-T29-7). Owner: maintainer; the local
  environment, not the repository.
- `build-the-container-image.md:61-66` says a stop needs the release of the queue's
  state; a sharper sentence that a large queue lengthens `docker stop -t` (T29 saw over
  10 s after about 45M rows) is outside this ticket (D-T29-8, low). Owner: maintainer.
- The in-process unit of an op is T30's definition, to be read as an order of magnitude
  (D-T30-2, low). Owner: maintainer.
- Nothing is measured on Linux (D-T30-3). Owner: maintainer or a later ticket.
- Closed-loop clients understate tail latency; an open-loop generator and a latency
  histogram are a later ticket (D-T30-4, low). Owner: maintainer.
- The `version` smoke bench from T16 stays until the maintainer removes it (D-T30-5,
  low).
- `just test-loadgen` runs only by hand; whether a gate runs it is open (D-T30-6).
  Owner: maintainer.
- `renovate.json`'s toolchain description says "four places" over five files, and
  `maintain-dependencies.md:82-83` says "three places" (D-T30-8, low). Owner:
  maintainer.
