---
id: T32
title: "Specification follow-up: expiry count on ack, empty pulls, and two new open questions"
status: done
depends_on: [T31]
parallel_with: [T33, T34]
branch: ticket/t32-spec-follow-up
estimated_size: M
---

# T32: Specification follow-up: expiry count on ack, empty pulls, and two new open questions

## Context

T31 integrated the MVP wave, T22 to T31, and handed back the questions its `code-review`
runs raised about the specification's behaviour as Open points for the maintainer
(`tickets/T31-mvp-integration.md`, "## Open points", lines 923 to 934). The maintainer has
answered two of them and left two open. This ticket carries all four into
`docs/specs/pachinko.allium` through the `spec-change` skill: the specification moves
first, then the scenarios, the obligations, the tests and the code.

| Item | Source in T31's Open points | Status | What this ticket does |
| --- | --- | --- | --- |
| Q4 | "Proposed open question (run 1, finding 5)", line 932: only a release or a revive resets `consecutive_expiries`, so a steadily drained pocket can park after three lapses | decided: an ack resets the pocket's `consecutive_expiries` to 0 | spec rule change, scenario S077, tests red first, then the code |
| Q3 | "Proposed open question (run 1, finding 4)", line 929: `PullWithinLease` extends the lease on an empty batch, so a consumer pulling in a loop holds an emptied pocket | decided as intended, documentation only | a spec comment and an HTTP API paragraph; no behaviour change |
| Q1 | "Proposed open question (`code-review` run 1, finding 1)", line 923: a supersede of a held record sheds the payload the consumer still holds, so a later nack dead-letters it too | not decided | open question 10 in the module, pointing to T35 |
| Q2 | "Proposed open question (run 1, finding 2)", line 926: `DropStalePublish` still touches the pocket, so a stale publish revives a parked pocket and resets its expiry count | not decided | open question 11 in the module, pointing to T35 |

Line numbers in this ticket are as of `4daf9df`, the head of pull request 40 (the MVP
wave); T32 starts from `main` after that pull request merges, so a line may have moved by
the merge, never by content.

**Q4, the expiry count.** The count lives on the `Pocket` entity
(`docs/specs/pachinko.allium` line 108) under `ExpiriesWithinLimit` (lines 138 to 140).
Today four rules reset it to 0: `ReviveIdlePocket` (line 349), `ReviveParkedPocket` (line
361), `ReleaseRequeuesPocket` (line 553) and `ReleaseIdlesEmptyPocket` (line 585); the
three lapse rules raise it (lines 611 to 664). The two ack rules,
`AckSettlesRecord` (lines 443 to 454) and `AckDefersNewerVersion` (lines 456 to 472),
already bind `let pocket = lease.pocket`, so each gains one clause,
`ensures: pocket.consecutive_expiries = 0`. The maintainer's decision names the ack only:
`NackSettlesRecord` (lines 474 to 496), `NackDefersNewerVersion` (lines 498 to 525),
`Renew` (lines 529 to 534) and the two pull rules keep the count as it is. That reading is
recorded in Open points as unconfirmed, and the tests pin it as a reading, not a decision.

In the library the count is `Pocket::consecutive_expiries`
(`crates/libpachinko/src/queue/pockets.rs` line 22), reset by `touch` (lines 64 to 79,
the revive arms) and `Pockets::release` (lines 242 to 254), raised by `Pockets::lapse`
(lines 265 to 283). Ack and nack share one path: `Queue::ack` (`settle.rs` lines 58 to
66) and `Queue::nack` (lines 95 to 117) both call `settle_held` (lines 122 to 147), which
checks the guards, extends the lease and calls `Pockets::settle` (`pockets.rs` lines 214
to 233). The reset therefore cannot live in `settle_held` or `Pockets::settle`
unconditionally; it belongs on the ack's side only, after every guard has passed, so that
a refused ack still changes nothing (S042).

The change makes two existing scenarios false as their tests are written, and the
invariant walk's oracle stale:

- S064, "Progress under a lease does not reset the expiry count"
  (`crates/libpachinko/tests/queue/expiry.rs` lines 397 to 408,
  `progress_under_a_lease_does_not_reset_the_expiry_count`), acks `a1`, renews, lapses and
  expects a count of 2; with the reset it is 1. S064 is re-pinned as "A pull, a nack or
  a renew under a lease does not reset the expiry count", and the ack's reset is the new
  scenario S077. Ids never compact, so S064 keeps its id with the new wording.
- S063, "At the threshold an empty pocket parks rather than idling" (`expiry.rs` lines 436
  to 457), empties the pocket under its third lease with two acks; with the reset the
  count falls to 0 and the lapse idles it. The test empties it with two nacks instead,
  which keeps the scenario true and is the second pin of the nack reading.
- The invariant walk (`crates/libpachinko/tests/queue/walk.rs`) predicts every step from
  a model written from the rules. Its `settle` (lines 569 to 611) is the oracle for both
  ack rules; without the reset `every_invariant_holds_after_every_step_of_any_walk`
  disagrees with the code once the code changes.

Every other test that acks reads a count of 0 or acks before a single lapse (S023 in
`crates/libpachinko/tests/queue/lease.rs` line 63, S061 at `expiry.rs` line 208, S075 at
line 662); the suite run in Step 6 is the check, not this list.

`docs/specs/pachinko.allium` lines 648 to 651, the comment above
`LeaseExpiryIdlesEmptyPocket`, says a pocket whose records were all acked or nacked parks
at the threshold; with the reset an emptied pocket reaches the threshold only if no ack
was processed since its earlier lapses, or when `max_consecutive_expiries` is 1. The
comment says so.

**Q3, the empty pull.** `PullWithinLease` (lines 378 to 390) ensures
`lease.expires_at = now + config.lease_ttl` whatever `batch` holds, so a pull that hands
out nothing still extends the lease; S036 (`crates/libpachinko/tests/queue/pull.rs` line
157) and S067's pull half (`expiry.rs` line 539,
`a_pull_after_expires_at_but_before_the_lapse_extends_the_lease`, an empty batch because
the record is still held) pin the extension. The maintainer's decision: this is intended,
no lease is granted on an empty pocket, and the client owns the release. The claim "no
lease is granted on an empty pocket" holds in the rules today, by reachability rather
than by an invariant:

- `PullClaimsFrontPocket` (lines 392 to 417) claims only a pocket with `status = queued`
  (lines 395 and 398), and `PullFindsNoPocket` (lines 429 to 435) answers when there is
  none.
- Every rule that makes a pocket queued creates or requires a record:
  `PublishIntoNewPocket` creates the pocket with one (lines 256 to 276);
  `ReviveIdlePocket` and `ReviveParkedPocket` (lines 341 to 363) answer `PocketTouched`,
  which only `PublishNewKey`, `Supersede` and `DropStalePublish` emit, each leaving the key
  a record; `ReleaseRequeuesPocket` (line 542) and `LeaseExpiryKeepsPocketAtFront`
  (line 636) guard on `pocket.records.count > 0`.
- Records leave a pocket only through `AckSettlesRecord` and `NackSettlesRecord` (`not
  exists record`, lines 453 and 495), which need an active lease, so the pocket is leased
  (`UnleasedHasNoActiveLease`, lines 130 to 132), never queued. An emptied leased pocket
  goes idle or parks when its lease ends (`ReleaseIdlesEmptyPocket`,
  `LeaseExpiryIdlesEmptyPocket`, `LeaseExpiryParksPocket`), and `IdleHasNoRecords`
  (lines 134 to 136) holds the idle side.

The wire page states the same: `docs/reference/http-api.md` lines 38 to 40 say a limit of
zero or less "claims a pocket and returns an empty batch", which is an empty batch from a
non-empty pocket, and lines 94 to 104 describe a pull within the lease as the lost-batch
recovery. No paragraph says a pull loop over an emptied pocket keeps the lease until the
client releases it.

**Q1 and Q2, the new open questions.** The module's open questions are lines 886 to 893,
under the comment at lines 880 to 884, which says the questions keep T19's numbers, the
eighth was closed by decision 0019, and "the last below is still the ninth". The numbering
does not compact (T31's Context; `docs/how-to/work-with-the-specs.md` lines 71 to 75), so
the two new questions are 10 and 11, appended after line 893, and the comment is rewritten
to say so. Today's behaviour for each is pinned by an existing scenario, which takes the
`oq:N` label the catalog uses for a placeholder (`docs/reference/testing.md` lines 154 to
156): S045 (`crates/libpachinko/tests/queue/settle.rs` line 158,
`two_supersedes_while_held_shed_both_losers_and_leave_the_held_version`) asserts the held
version is shed at the first supersede, which is question 10's behaviour, and S024
(`expiry.rs` line 271, `a_supersede_or_a_stale_publish_revives_a_parked_pocket`) asserts a
stale publish revives a parked pocket at count 0, which is question 11's. The label
follows S026's form (`expiry.rs` lines 310 to 311). The handbook counts the open
questions in three places: `docs/how-to/work-with-the-specs.md` lines 63 to 85 ("carries
eight today" and the list), `docs/explanation/specifications.md` lines 56 to 66 ("The
root carries eight") and `CHANGELOG.md` lines 16 to 19 ("keeps eight open questions",
T22's entry, which stays as T22's record beside this ticket's new line).

**Obligations.** T31's Context records that `just plan-spec pachinko` printed 190
obligations at `611a86e`; the only specification commit since is `0590a30` (T22), whose
hand-back records 190 again. T26's hand-back quotes 192, which T22's and T31's 190 do not
match; this ticket counts the `obligations` array with `jq '.obligations | length'`, so
its figure is comparable with theirs, and says how it counted. An added
`ensures` may add ids (the module's `when-set.*` ids come from such clauses) or none, and
an open question adds none. This ticket does not predict the after count: it records both
counts and both id sets, and every id that is new or changed is tagged by a test
(`docs/reference/testing.md`, "Tests from the specifications", owns the tag rules). The
obligation ids this ticket names below are the grammar the code already uses
(`rg -n '// obligation:' crates`): `rule-success.<Rule>`, `rule-failure.<Rule>.<n>`,
`invariant.<Entity>.<Name>`, `when-set.<Rule>.<Entity>.<field>`; each is confirmed against
`just plan-spec pachinko` at execution time, and one the checker does not print is not
tagged.

**Risks** (`CONVENTIONS.md` §13). Ticket and handbook prose pass typos and lychee runs
offline over `tickets/`, so paths stay code spans. `walk.rs` is the suite's longest test
file (1,168 lines at `4daf9df`) under `rustqual.toml`'s test-file limit (line 68), so the
oracle change is kept to a few lines and `just metrics` runs. T33 and T34 run beside this
ticket on the wave-2 rule (`CONVENTIONS.md` §11): shared files are edited on own lines
only, and `origin/main` is merged before the push.

## Goal

The specification says that an ack resets its pocket's `consecutive_expiries` to 0, says
why a within-lease pull extends the lease on an empty batch and that no lease is granted
on an empty pocket, and carries open questions 10 and 11 with today's behaviour stated as
a placeholder; the library resets the count on an ack and on nothing else new, shown red
first and shown to bite; scenario S077 pins the reset and S064 and S063 are re-pinned to
the new rules; the walk's oracle follows the rules; every obligation id the change adds
is tagged by a test, with the before and after counts recorded; and the HTTP API page,
the testing catalog and the open-question lists match.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rule clauses (2) | `ensures: pocket.consecutive_expiries = 0` in `AckSettlesRecord` and `AckDefersNewerVersion` |
| Spec comments (4) | the ack paragraph in "Ack and nack"; the `PullWithinLease` comment; the `LeaseExpiryIdlesEmptyPocket` comment; the Open Questions numbering comment |
| Open questions (2) | 10, the held version shed at supersede (Q1); 11, a stale publish touching its pocket (Q2) |
| Scenarios | S077, new; S064, reworded and re-pinned; S063, re-pinned; S024 and S045, labelled `oq:11` and `oq:10` |
| Obligations | every id `just plan-spec pachinko` prints after the edit that it did not print before, plus `rule-success.AckSettlesRecord` and `rule-success.AckDefersNewerVersion` on S077 |

The new scenario:

| Id | Scenario | Tags |
| --- | --- | --- |
| S077 | An ack resets the consecutive-expiry count | edge |

## Non-goals

- No answer to Q1 or Q2. Open questions 10 and 11 record today's behaviour as a
  placeholder; T35 decides them. `Supersede`, `DropStalePublish` and the nack rules keep
  their clauses, and S024 and S045 keep their assertions.
- No reset on a nack, a renew or a pull. The decision names the ack; whether a nack should
  reset is an Open point, not a change here.
- No behaviour change for Q3: `PullWithinLease` keeps its clauses and the library's pull
  path is untouched.
- No new invariant. "A queued pocket has a record" holds by reachability today; stating
  it as an invariant would add obligations and is an Open point.
- No renumbering of the open questions, and no edit to the eight that exist.
- No service code: the HTTP surface already carries the ack, so the reset reaches the wire
  through the library. No new route, flag or error code.
- No change to T30's figures: workload W5 (`crates/libpachinko/benches/workloads.rs` lines
  29 to 37) never acks, so its parking cycle is unchanged.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | The two ack clauses; the ack paragraph after lines 437 to 441; the `PullWithinLease` comment above line 378; the comment at lines 648 to 651; the numbering comment at lines 880 to 884; open questions 10 and 11 after line 893 |
| `crates/libpachinko/src/queue/settle.rs` | shared, own lines | `Queue::ack` resets the count after `settle_held` succeeds; its doc (lines 25 to 30) says so; the module doc names the reset |
| `crates/libpachinko/src/queue/pockets.rs` | shared, own lines | One `pub(super)` method that resets a pocket's count, documented with its rule names; nothing else |
| `crates/libpachinko/tests/queue/expiry.rs` | shared, own lines | S077's test, new; S064's test rewritten and renamed; S063's test empties the pocket with nacks; S024's scenario label |
| `crates/libpachinko/tests/queue/settle.rs` | shared, own lines | S045's scenario label |
| `crates/libpachinko/tests/queue/walk.rs` | shared, own lines | The model's `settle` (lines 569 to 611) resets the count on an ack; the driver's `settle` (lines 924 to 964) reports the two reach labels |
| `crates/libpachinko/tests/queue/invariants.rs` | shared, own lines | Two labels in `WANTED` (lines 371 to 433): an ack and a nack on a pocket with a count above 0 |
| `crates/libpachinko/tests/queue/fixture.rs` | shared, own lines | Only if S077 needs a helper the fixture lacks; otherwise untouched, and the hand-back says which |
| `docs/reference/http-api.md` | shared, own lines | The empty-pull paragraph after lines 56 to 68; the ack reset in that paragraph; "lapses in a row" at lines 73 to 74 |
| `docs/reference/testing.md` | shared, own lines | "The wave's 76 scenarios" (line 152); rows S024, S045 and S064; a row for S077 |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | Lines 63 to 85: ten questions, the list with 10 and 11 |
| `docs/explanation/specifications.md` | shared, own lines | Lines 56 to 66: ten questions and where the last two came from |
| `docs/explanation/architecture.md` | shared, own lines | Line 196, "its pocket's expiry count rises": what resets it |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Changed: the ack reset and the two open questions; pachinko Unreleased, a Changed heading with the reset as the wire sees it |
| `tickets/README.md` | shared, own lines | T32's Status cell |
| `tickets/T32-spec-follow-up.md` | ticket | `status:` and hand-back notes |

T33 and T34 may edit `CHANGELOG.md`, `tickets/README.md` and the handbook pages above;
this ticket edits only its own lines in each.

## Steps

1. Create the worktree from `main` after T31 has merged (`README.md`, "How to pick up a
   ticket"), `git worktree add ../T32 -b ticket/t32-spec-follow-up main` or its Supacode
   form (`CONVENTIONS.md` §11), and run `just initialize` there (network, authorised by
   this step); it installs the pinned Allium checker, without which `just check-specs`
   cannot pass (`AGENTS.md`). In a clean worktree already initialized on `main` whose
   branch carries nothing, create the branch there and skip `just initialize`. Run
   `just check` on the untouched worktree and quote its last line as the baseline,
   headed by the commit; a red baseline stops the ticket (`CONVENTIONS.md` §11).
2. **The plan.** Use the `plan-change` skill. Read the whole of
   `docs/specs/pachinko.allium`, then the library and test lines Context cites, and
   confirm each line reference still points where Context says. Record the obligations
   before any edit:

   ```sh
   mkdir -p ai_tmp/T32
   set -o pipefail
   just plan-spec pachinko > ai_tmp/T32/plan-before.json; echo "exit $?"
   jq '.obligations | length' ai_tmp/T32/plan-before.json; echo "exit $?"
   jq '.diagnostics | length' ai_tmp/T32/plan-before.json; echo "exit $?"
   ```

   Quote the count (190 is expected; a different count is recorded and explained, not
   corrected). Re-derive the Q3 reachability argument in Context against the module with
   `rg -n 'status = queued|not exists record|records.count > 0' docs/specs/pachinko.allium`
   and `rg -n 'PocketTouched' docs/specs/pachinko.allium`; if any rule makes a pocket
   queued without a record, stop: the documentation the maintainer decided on would be
   false, and that goes back to the maintainer before Step 3.
3. **The specification.** Use the `spec-change` skill, which hands Allium syntax to the
   `allium` skill. Edit `docs/specs/pachinko.allium`:
   - `AckSettlesRecord` and `AckDefersNewerVersion` each gain
     `ensures: pocket.consecutive_expiries = 0`, placed after the lease's extension.
     Guards, other clauses and rule order are unchanged, so the two rules stay disjoint on
     `held_rank` against `rank`.
   - After the "Ack and nack" comment (lines 437 to 441), a paragraph:

     ```text
     -- An ack is progress: both ack rules reset the pocket's
     -- consecutive_expiries to 0, so lapses are consecutive only while no
     -- ack, release or revive comes between them. A nack, a renew and a pull
     -- leave the count; the decision names the ack only.
     ```

   - Above `PullWithinLease` (line 378), after the pull section's comment:

     ```text
     -- PullWithinLease extends the lease whatever the batch holds, an empty
     -- batch included: a consumer that pulls in a loop keeps its pocket after
     -- acking or nacking every record, and releasing it is the consumer's
     -- action. That is intended. No lease is granted on an empty pocket:
     -- PullClaimsFrontPocket claims only a queued pocket, every rule that
     -- makes a pocket queued creates a record or requires one, and a pocket
     -- loses records only under an active lease, while it is leased.
     ```

   - The comment above `LeaseExpiryIdlesEmptyPocket` (lines 648 to 651) keeps its two
     sentences and adds: "An ack resets the count, so an emptied pocket reaches the
     threshold only if no ack was processed since its earlier lapses, or when
     max_consecutive_expiries is 1."
   - The numbering comment (lines 880 to 884) says that the questions keep their numbers
     in the order below; that T19 numbered the first nine and the eighth was closed by
     decision 0019 (see the Lease expiry comment), so the eighth line below is question 9,
     which scenario tags and `OneVisitPerKeyPerLease` name so; and that the ninth and
     tenth lines are questions 10 and 11, added from the T31 review.
   - After line 893, the two new questions, each one line:

     ```text
     open question "A supersede of a held record sheds the version the consumer still holds: Supersede copies record.payload, which is the held payload while the held rank is current, into a SupersededRecord, and a later nack of that held version (NackDefersNewerVersion) makes a DeadLetter of the same payload, so it sits in both shed queues. Should the held version be shed when it is settled rather than when it is superseded, and should a nack of a superseded held version still dead-letter it? The rules do both today; that is a placeholder, not a decision. Ticket T35 decides it."
     open question "A stale publish touches its pocket: DropStalePublish emits PocketTouched like every publish into an existing pocket, so a stale publish into a parked pocket revives it through ReviveParkedPocket and resets its consecutive_expiries although no record changed. Should a stale publish touch the pocket? It does today; that is a placeholder, not a decision. Ticket T35 decides it."
     ```

     No URL goes in the module.
   Run `just check-specs` and `just analyse-specs`; both must be green, and a diagnostic
   or finding is fixed at its root (the `spec-change` skill, step 7). Then record the
   obligations after the edit and the id sets (the Verification block's obligation
   lines), and quote both counts and the `diff` of the id sets, headed by the commit.
4. **Adversarial review** (`CONVENTIONS.md` §14), checkpoint T32-1, before any test or
   code follows the specification: the specification diff, the plan-spec before and
   after counts and the id `diff`, against Context, Goal and Non-goals. Ask: whether the
   two ack rules now reset the count and no other rule changed a guard or a clause;
   whether the rules remain disjoint and `ExpiriesWithinLimit` still holds on every
   path; whether any nack, renew or pull rule resets the count; whether the
   `PullWithinLease` comment's "no lease is granted on an empty pocket" is true of every
   rule that makes a pocket queued, checked rule by rule; whether the
   `LeaseExpiryIdlesEmptyPocket` comment is true at `max_consecutive_expiries` 1 and
   above; whether open questions 10 and 11 state today's behaviour exactly, as a
   placeholder and not a decision, without a URL; whether the numbering comment names
   the right line for each number; whether any obligation id changed or vanished beyond
   the ones the new clauses explain. Two reviewer subagents with fresh contexts run in
   parallel, one lens each, `conformance` and `breakage`, each on the harness §14 picks
   (the one the executing agent is not, if it is on `PATH`; otherwise its own): from
   Claude Code, `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T32-1/findings-<lens>.md "<brief>" < /dev/null` (§14's command; `-o`
   writes only the reviewer's last message, and a read-only reviewer writes no file of
   its own); from Codex, `claude -p --model claude-opus-5-5 --permission-mode plan
   "<brief>" > ai_tmp/review/T32-1/findings-<lens>.md`, run from the worktree; separate
   triage subagents check every finding against the tree and write
   `ai_tmp/review/T32-1/verdicts.md`, each finding valid with its fix or invalid with the reason; one writer applies the valid
   fixes one at a time, only within Files touched, and hands back any fix outside it; if
   a high-severity finding was valid, a second round reviews the fix diff and writes
   beside the first with a `-2` suffix; two rounds at most. Rerun `just check-specs`,
   `just analyse-specs` and the obligation lines after the fixes.
5. **The tests, red first.** Use the `rust-change` skill. Before any library code:
   - S077, `an_ack_resets_the_consecutive_expiry_count` in `tests/queue/expiry.rs`,
     beside S056 and S064, built on `lapsed_once()` (count 1 on pocket `a`): pull `a` as
     `c1`; an ack of a key the pocket does not hold is `RecordNotHeld` and the count is
     still 1; an ack of `a1` is `Settled` and the count is 0 while the pocket is leased;
     the lapse makes it 1 and keeps `a` queued at the front. Then the deferred half: pull
     `a` again, publish `a2` at rank 2 while held, ack `a2`, which is `Deferred`, and the
     count is 0; from there the pocket parks only on the third lapse after that ack, not
     the second. Tags: `// scenario: S077`, `// obligation:
     rule-success.AckSettlesRecord`, `// obligation: rule-success.AckDefersNewerVersion`,
     `// obligation: invariant.Pocket.ExpiriesWithinLimit`, and each new id from Step 3's
     `diff` that this test covers.
   - S064, renamed `a_pull_a_nack_or_a_renew_under_a_lease_does_not_reset_the_expiry_count`:
     from `lapsed_once()`, pull `a`, nack `a1`, renew, pull again under the same lease;
     the count stays 1 throughout, and the lapse makes it 2. Keep its tags and add
     `rule-success.NackSettlesRecord` if no test closer to the nack carries it.
   - S063, same name: under the third lease, nack `a1` and `a2` where it acked them, so
     the pocket is empty at the threshold and parks with count 3, as before.
   - The walk: in `walk.rs`'s model `settle` (lines 569 to 611), after the lease's
     extension, an ack sets the model pocket's `consecutive_expiries` to 0; a nack
     leaves it. The driver's `settle` (lines 924 to 964) reports `ack resets a counted
     pocket` when an ack succeeds on a pocket whose count was above 0 before the step,
     and `nack keeps a counted pocket` likewise for a nack; both labels go in `WANTED`
     in `invariants.rs`, so `the_walk_reaches_every_state_and_every_outcome` fails if the
     generator never reaches either. If it fails to reach one at the walk's
     `max_consecutive_expiries` of 2 (`walk.rs` line 60), adjust the generator's weights
     or the walk's length; never drop the label.
   - Labels only, no assertion changes: S024's tag line becomes
     `// scenario: S024 (open question 11: the stale half, as the rules stand)` and
     S045's `// scenario: S045 (open question 10: the held version is shed at the
     supersede, as the rules stand)`, each wrapped as S026's is.
   Run `NEXTEST_PROFILE=ci just test` (the `ci` profile in `.config/nextest.toml` does not
   fail fast, so one run reports every failure; green runs keep plain `just test`) and
   quote, headed by the commit with uncommitted changes, the failures this must show
   before any code: S077 on its first count assertion (1, not 0), and the walk's
   `every_invariant_holds_after_every_step_of_any_walk` on an ack prediction. S063 and the
   new S064 pass already, as they pin what the code already does; Step 6 shows they bite.
   If anything else goes red, read it against the decision: a test that pinned the old
   reading is changed and listed in the hand-back with the reason; any other failure is a
   finding.
6. **The code.** Use the `rust-change` skill. Add the reset method to `Pockets` in
   `pockets.rs`, documented with `AckSettlesRecord` and `AckDefersNewerVersion`, and call
   it from `Queue::ack` in `settle.rs` once `settle_held` has returned `Ok`, never from
   `settle_held`, `Queue::nack` or `Pockets::settle`. Update `Queue::ack`'s doc (lines 25
   to 30) and the module doc, and `Queue::nack`'s doc (lines 68 to 72): its "as
   [`Queue::ack`]" would otherwise carry the reset, so it says a nack settles as an ack
   does but leaves the pocket's consecutive-expiry count. Run `just test`: S077 and the
   walk are green. Then show each test bites, on scratch edits never committed,
   restoring the source after each and quoting the failure (`docs/reference/testing.md`, "A test that passed when first run
   is shown to bite"); each bite run uses `NEXTEST_PROFILE=ci just test`, so every test
   it names reports:
   - the reset removed: S077 and the walk fail;
   - the reset moved into `settle_held`, so a nack resets too: S063, S064 and the walk
     fail;
   - the reset moved before the `RecordNotHeld` guard, so a refused ack resets: S077
     fails;
   - a reset added to `Queue::renew`: S064 fails; one added to the pull within a lease:
     S064 fails;
   - the two `WANTED` labels with their `turn.reach` calls removed from the driver:
     `the_walk_reaches_every_state_and_every_outcome` fails, naming them.
   Run `just fmt-check`, `just clippy`, `just test`, `just doc`, `just coverage`,
   `just effects` and `just metrics`.
7. **The handbook.** Use the `review-docs` skill; each page below owns its topic in
   `docs/manifest.yml`, so the text goes there and nowhere else.
   - `docs/reference/http-api.md`, after the fencing paragraph (lines 56 to 68): "A pull
     under a lease extends it whatever the batch holds. A pull that returns no records,
     because `limit` is zero or less or the pocket's records are all held or were all
     acked or nacked, still moves
     `expires_at` to now plus `--lease-ttl-ms` and keeps the pocket, so a consumer that
     pulls in a loop holds its pocket, emptied or not, until it releases the lease; the
     release is the client's to send. No pull claims an empty pocket: a pull without a
     lease claims only a queued pocket, which always has a record, and answers
     `no_pocket` when there is none." In the fencing paragraph, one sentence that an ack
     also resets the pocket's count of consecutive lapses, and a nack does not; in the
     lapse paragraph (lines 73 to 74), "lapses in a row" becomes lapses with no ack,
     release or publish revival between them.
   - `docs/reference/testing.md`: "The wave's 76 scenarios" becomes 77; the S064 row
     takes the new description and test name; S024's tags become `edge, oq:11` and S045's
     `edge, oq:10`, each description ending with the open question as the rules stand,
     as S026's does; a row for S077 after S076.
   - `docs/how-to/work-with-the-specs.md` lines 63 to 85: "carries eight today" and "The
     eight that remain" say ten, and the list gains "Question 10, a supersede of a held
     record shedding the version the consumer still holds." and "Question 11, a stale
     publish reviving a parked pocket."
   - `docs/explanation/specifications.md` lines 56 to 66: ten questions, the two new ones
     named in the list, and their origin: the wave's code review exposed them after the
     implementation, as writing the rules exposed the ninth.
   - `docs/explanation/architecture.md` line 196: the expiry count rises on a lapse and
     an ack, a release or a revive resets it.
   - `CHANGELOG.md`: under libpachinko Unreleased, Changed, one bullet: `Queue::ack`
     resets its pocket's consecutive-expiry count, so only `max_consecutive_expiries`
     lapses with no intervening ack, release or revive park a pocket; lapses before an
     ack no longer count toward it; a nack, renew or pull leaves
     the count; the specification gains open questions 10 and 11, ten in all, and says a
     pull within a lease extends it on an empty batch. Under pachinko Unreleased, a
     `### Changed` heading above `### Added` with one bullet: an ack over HTTP resets the
     count `--max-consecutive-expiries` measures.
   Run `just check-docs`.
8. **Reconcile the obligations.** Run the Verification block's obligation lines. Every id
   in `ai_tmp/T32/ids-new.txt` has at least one `// obligation:` tag under `crates/`; an
   id with none is tagged on the test that exercises it, or, if no test can observe it,
   listed in the hand-back with the reason, as T26 listed `rule-failure.LeaseExpiry*.3`.
   An id in `ids-before.txt` and not in `ids-after.txt` that a test still tags is removed
   from that tag. The hand-back quotes the before count, the after count, the `diff` and
   each new id with the test that carries it.
9. **Adversarial review** (`CONVENTIONS.md` §14), checkpoint T32-2, before the final
   commit: the whole diff of this ticket against Context, Goal, Non-goals, Files touched,
   the acceptance criteria and §11. Ask: whether the code resets the count on exactly
   the two ack rules and on no refused ack; whether S077, S064, S063 or the walk would
   pass with the reset removed, with a nack reset added, or with a renew or pull reset
   added; whether the walk's oracle was changed to agree with the code rather than with
   the rules; whether the reach labels can be satisfied without an ack on a counted
   pocket; whether any assertion of S024 or S045 changed; whether any new obligation id
   is untagged or tagged on a test that does not exercise it; whether the HTTP API
   paragraph says anything the rules do not; whether a page still says eight open
   questions or 76 scenarios; whether a Verification line can print a pass when it
   failed; and whether any file outside Files touched changed. Two reviewer subagents
   with fresh contexts run in parallel, one lens each, `conformance` and `breakage`, each
   on the harness §14 picks (the one the executing agent is not, if it is on `PATH`;
   otherwise its own): from Claude Code, `codex exec -m gpt-6-astra -s read-only -C
   <worktree> -o ai_tmp/review/T32-2/findings-<lens>.md "<brief>" < /dev/null`; from
   Codex, `claude -p --model claude-opus-5-5 --permission-mode plan "<brief>" >
   ai_tmp/review/T32-2/findings-<lens>.md`, run from the worktree; separate triage
   subagents write `ai_tmp/review/T32-2/verdicts.md`; one writer applies the valid fixes
   one at a time, only within Files touched, and hands back any fix outside it; a valid
   high-severity finding takes a second round over the fix diff with a `-2` suffix; two
   rounds at most. Rerun the Verification block after the fixes.
10. **Review, merge and gate.** Use the `review-change` skill over the diff, then the
    `project-check` skill: `just check`, with `fix-quality` for any failing recipe. Run
    `git fetch origin` (network, authorised by this step) and merge `origin/main`
    (`CONVENTIONS.md` §11): T33 or T34 may have merged since this branch was cut, so a
    conflict in a shared file keeps both tickets' lines. Rerun the Verification block at
    the merge commit and requote it. Set `status: done` here and in T32's row of
    `tickets/README.md`, and commit on the ticket branch with the hand-back notes, each
    quoted output headed by the commit it ran at. Pushing and opening the pull request are
    separately authorised: stop and ask.

## Acceptance criteria

- `AckSettlesRecord` and `AckDefersNewerVersion` each ensure
  `pocket.consecutive_expiries = 0`; no nack, renew or pull rule resets the count; and
  `just check-specs` and `just analyse-specs` are green.
- The module carries the ack paragraph, the `PullWithinLease` comment, the amended
  `LeaseExpiryIdlesEmptyPocket` comment and the amended numbering comment, and
  `rg -c '^open question ' docs/specs/pachinko.allium` prints 10; questions 10 and 11
  state today's behaviour as a placeholder and name T35, and the module holds no URL that
  was not there before.
- The hand-back quotes `just plan-spec pachinko`'s obligation count before and after, the
  `diff` of the two id sets, and, for each new id, the test that tags it; no new id is
  untagged without a stated reason.
- S077 failed before the code and passes after; the walk failed on an ack prediction
  before the code and passes after; the hand-back quotes both failures.
- Each break in Step 6 makes the tests it names fail, quoted in the hand-back.
- The scenario id set over `crates/libpachinko/tests` and `crates/libpachinko/src` is
  exactly S001 to S077 with no id tagged twice, and `docs/reference/testing.md`'s catalog
  has 77 rows, S064 reworded and S024 and S045 labelled.
- `docs/reference/http-api.md` states the empty-pull rule, that no pull claims an empty
  pocket, that the client owns the release and that an ack resets the lapse count; no
  file under `docs/` (outside `docs/decisions/`) or `crates/` says eight open questions
  or 76 scenarios. `CHANGELOG.md` lines 16 to 19 and 31 stay as T22's and T28's record,
  beside this ticket's own bullet.
- `just fmt-check`, `just clippy`, `just test`, `just doc`, `just effects`,
  `just metrics`, `just check-docs` are green, and `just coverage` reports at or above
  90% of lines over `crates/*/src/**`.
- `just check` is green at the commit that merges `origin/main`, and `git status --short`
  prints nothing after the ticket's commit.
- The two adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding
  is fixed or handed back.

## Verification

Each command runs from the worktree root under `set -o pipefail`; each line that a claim
rests on shows its exit status. `ai_tmp/T32/plan-before.json` is Step 2's.

```sh
set -o pipefail
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
just plan-spec pachinko > ai_tmp/T32/plan-after.json; echo "exit $?"
jq '.obligations | length' ai_tmp/T32/plan-before.json ai_tmp/T32/plan-after.json; echo "exit $?"
jq '.diagnostics | length' ai_tmp/T32/plan-after.json; echo "exit $?"
jq -r '.obligations[].id' ai_tmp/T32/plan-before.json | sort > ai_tmp/T32/ids-before.txt; echo "exit $?"
jq -r '.obligations[].id' ai_tmp/T32/plan-after.json | sort > ai_tmp/T32/ids-after.txt; echo "exit $?"
wc -l ai_tmp/T32/ids-before.txt ai_tmp/T32/ids-after.txt; echo "exit $?"
diff ai_tmp/T32/ids-before.txt ai_tmp/T32/ids-after.txt; echo "exit $? (1 when the sets differ)"
comm -13 ai_tmp/T32/ids-before.txt ai_tmp/T32/ids-after.txt > ai_tmp/T32/ids-new.txt; echo "exit $?"
wc -l < ai_tmp/T32/ids-new.txt; echo "exit $?"
while read -r id; do rg -n --fixed-strings "obligation: $id" crates || echo "UNTAGGED $id"; done < ai_tmp/T32/ids-new.txt
rg -n 'scenario: S077' -A6 crates/libpachinko/tests/queue/expiry.rs; echo "exit $?"
rg -c '^open question ' docs/specs/pachinko.allium; echo "exit $?"
rg -n 'T35' docs/specs/pachinko.allium; echo "exit $?"
# positive control for the scenario lines below: it must exit 0 and count tags
rg -c 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src; echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort -u | diff - <(printf 'scenario: S0%02d\n' $(seq 1 77)); echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort | uniq -d; echo "exit $?"
rg -c '^\| S0[0-9]{2} \|' docs/reference/testing.md; echo "exit $?"
# the stale-count search: its positive control is the same search on the base commit,
# which must print lines; then on the branch it must print nothing and exit 1
base=$(git merge-base HEAD origin/main)
git diff "$base" -- docs/specs/pachinko.allium docs/reference/http-api.md; echo "exit $?"
git diff "$base" -- docs/specs/pachinko.allium > ai_tmp/T32/spec-diff.txt; echo "exit $? (must be 0)"
rg -n '^\+.*https?://' ai_tmp/T32/spec-diff.txt; echo "exit $? (1 when the module gains no URL)"
# Q3, documentation only: the new comment above PullWithinLease and the empty-pull paragraph
rg -n -B8 '^rule PullWithinLease' docs/specs/pachinko.allium | rg -n 'empty pocket'; echo "exit $? (must be 0)"
rg -n -i 'empty batch' docs/reference/http-api.md; echo "exit $? (must be 0)"
git grep -n -E 'carries eight|eight that remain|root carries eight|76 scenarios|progress_under_a_lease' "$base" -- docs crates ':!docs/decisions'; echo "exit $?"
rg -n 'carries eight|eight that remain|root carries eight|76 scenarios|progress_under_a_lease' docs crates --glob '!docs/decisions/*'; echo "exit $?"
just fmt-check; echo "exit $?"
just clippy; echo "exit $?"
just test; echo "exit $?"
just doc; echo "exit $?"
just effects; echo "exit $?"
just metrics; echo "exit $?"
just coverage; echo "exit $?"
just check-docs; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Hand-back notes

### Baseline

At `53d64b7` (the head of `wave-2-tickets`), in a new worktree on
`ticket/t32-spec-follow-up`, `just initialize` exited 0. It installed allium 3.6.1 and,
because this is a secondary worktree, skipped `install-hooks`. `just check` on the
untouched worktree exited 0:

```text
All checks passed and the worktree is unchanged.
```

Commits on the branch:

| Commit | Content |
| --- | --- |
| `41e5370` | Step 3: the specification |
| `dd4c68f` | the T32-1 fixes |
| `2158750` | Steps 5 and 6: tests and code |
| `b0e5c30` | Step 7: the handbook |
| `7d637ac` | the T32-2 fix |
| `2681bd6` | the `code-review` fixes |

The final commit carries this section and `status:`.

### Overrides of the ticket's assumptions

- **Base.** Pull request 40 had not merged, so `main` (`611a86e`) holds no MVP code. The
  maintainer chose to cut the branch from `wave-2-tickets` (`53d64b7`), stacked on pull
  request 41.
  - At Step 2, `origin/wave-2-tickets` did not exist, so every pre-merge diff, the
    review briefs' and the Verification block's `base=` used the branch point `53d64b7`
    literally.
  - `merge-base HEAD origin/main` would be `611a86e`, which would pull in the whole MVP
    diff.
  - After Step 10's fetch, `merge-base HEAD origin/wave-2-tickets` is `53d64b7`, the same
    commit.
- **Triage (§14 phase 2).** By the maintainer's choice, findings were not triaged by
  separate subagents. The implementer agent that wrote the work under review checked each
  finding against the tree, wrote the verdicts and applied the valid fixes one at a time.
  The lead triaged the one finding on the ticket file. The lead and three implementer
  agents (spec, Rust, handbook) ran the steps.
- **Line drift since `4daf9df`.** Content was unchanged; these lines moved:
  - `testing.md` "76 scenarios" moved from line 152 to 171.
  - `architecture.md` "expiry count rises" moved from 196 to 223.
  - T22's `CHANGELOG.md` entry moved to lines 34 to 37, and T28's line to 54.
  - In `settle.rs`, `ack`, `nack` and `settle_held` are at 60, 100 and 127.
  - In `walk.rs`, the model's `settle` is at 570 and the driver's at 923; the file has
    1,167 lines.
- **Files outside Files touched, each a minimal edit.** `docs/explanation/prior-art.md`,
  `README.md` and `crates/libpachinko/src/queue/inspect.rs` are outside Files touched.
  Each still said something the change makes false:
  - `prior-art.md:229` said "carries eight open questions", which would fail the
    acceptance search. It came with commit `189b76e`, after the ticket was written.
  - `README.md:108` said "lapses three times in a row".
  - `PocketView::consecutive_expiries` was documented as "in a row".
- **Wording departures from the ticket's verbatim text** (reasons under T32-1 and
  `code-review`):
  - Five spec departures:
    - the `PullWithinLease` comment, which now names the lapse qualifier and how the
      revive rules reach a record;
    - the ack paragraph's last clause;
    - open question 10's "shed twice";
    - open question 11's "like PublishNewKey and Supersede";
    - the numbering comment, which the ticket describes but does not word.
  - In the handbook, the http-api empty-pull paragraph carries the lapse qualifier and
    names deferred records.
  - S045's catalog suffix reads "(OQ10 as the rules stand: the first loser is the version
    still held)", because a bare label would contradict "the held version is untouched".
  - `specifications.md` drops "as writing the rules exposed the ninth", because the
    sentence already names question 9's origin.
- **`CHANGELOG.md` line 59, T26's Added bullet.** "lapses in a row" became "lapses with no
  ack, release or revive between them" (a `code-review` fix). T22's and T28's lines stay,
  as the acceptance criteria require.
- **`just fix`.** The Rust agent ran it once. It stripped trailing whitespace in
  `tickets/T30-performance-evidence.md`, which was restored with `git checkout --` that
  file. It changed nothing else.

### Step 2: the plan and the obligations before

At `53d64b7`:

```text
$ just plan-spec pachinko > ai_tmp/T32/plan-before.json; echo "exit $?"
exit 0
$ jq '.obligations | length' ai_tmp/T32/plan-before.json
190
$ jq '.diagnostics | length' ai_tmp/T32/plan-before.json
0
```

The count is 190, as T22 and T31 recorded. It was counted with
`jq '.obligations | length'`.

The Q3 argument was re-derived with the ticket's two `rg` searches. A pocket becomes
queued at five places:

- line 263, `PublishIntoNewPocket`, which creates a record;
- lines 347 and 359, the revive rules, which answer `PocketTouched`. Only lines 296
  (`PublishNewKey`), 319 (`Supersede`) and 338 (`DropStalePublish`) emit it, and each
  leaves a record;
- line 551, `ReleaseRequeuesPocket`, guarded at 542;
- line 645, `LeaseExpiryKeepsPocketAtFront`, guarded at 636.

Records leave only at lines 453 and 495, under an active lease. The argument holds, so
nothing stopped.

### Step 3: the specification

At `41e5370`:

- `just check-specs` exited 0 and `just analyse-specs` exited 0, with no diagnostics and
  no findings.
- Obligations were 190 before and 190 after, and plan-after has 0 diagnostics.
- `diff ids-before.txt ids-after.txt` printed nothing (exit 0), and `ids-new.txt` has 0
  lines.
- `rg -c '^open question '` prints 10.

The new `ensures` adds no id: this module's `when-set.*` ids come only from fields
declared `when status = ...`. Twelve ids keep their id and description but change content,
because `entities_written` gains `Pocket`:

- `rule-success.AckSettlesRecord` and `rule-success.AckDefersNewerVersion`;
- `rule-failure.AckSettlesRecord.1` to `.5`;
- `rule-failure.AckDefersNewerVersion.1` to `.5`.

All twelve were already tagged, and S077 carries the two `rule-success` ids.

### T32-1: the specification (step 4)

The reviewers were `codex exec -m gpt-6-astra -s read-only`, one per lens, over
`git diff 53d64b7 -- docs/specs/pachinko.allium` at `41e5370` and the plan-spec files.

- **Conformance:** no findings on any of the eight questions.
- **Breakage B1 (medium), valid:** a looping pull keeps the pocket only if each pull is
  processed before the lapse, and at `lease_ttl` 0 it never does. Fixed: the
  `PullWithinLease` comment adds "as long as each pull is processed before the lease
  lapses (see the Lease expiry comment)". The same qualifier went into http-api.md.
- **B2 (low), valid:** the superseded copy can decay before the nack, so "sits in both shed
  queues" is not guaranteed. Fixed: open question 10 says "so that payload is shed twice,
  once into each shed queue".
- **B3 (low), valid:** a publish at the stored rank matches no rule and emits nothing.
  Fixed: open question 11 says "like PublishNewKey and Supersede".

All three fixes are in `dd4c68f`. There was no valid high finding, so there was no second
round. The comment above `PublishNewKey` (line 278, written before T32) has B3's
imprecision and is handed back under Open points.

The rerun at `dd4c68f` with uncommitted changes: `check-specs` and `analyse-specs` exited
0, obligations stayed at 190 with an empty id diff, and there are 10 open questions.

### Step 5: the tests, red first

At `dd4c68f` with uncommitted test changes, nothing under `src` changed:

```text
$ NEXTEST_PROFILE=ci just test; echo "exit $?"
exit 100
     Summary [   1.146s] 242 tests run: 239 passed, 3 failed, 0 skipped
        FAIL ( 49/242) libpachinko::queue expiry::an_ack_resets_the_consecutive_expiry_count
        FAIL (106/242) libpachinko::queue invariants::the_walk_reaches_every_state_and_every_outcome
        FAIL (118/242) libpachinko::queue invariants::every_invariant_holds_after_every_step_of_any_walk
    thread 'expiry::an_ack_resets_the_consecutive_expiry_count' panicked at crates/libpachinko/tests/queue/expiry.rs:432:5:
    assertion `left == right` failed: a settling ack
      left: (Leased, 1)
     right: (Leased, 0)
    thread 'invariants::every_invariant_holds_after_every_step_of_any_walk' panicked at crates/libpachinko/tests/queue/walk.rs:1120:5:
    assertion `left == right` failed: the state the rules predict
      left: ... PocketKey("a"): ModelPocket { status: Leased, position: Position(0), consecutive_expiries: 1, records: {} } ...
     right: ... PocketKey("a"): ModelPocket { status: Leased, position: Position(0), consecutive_expiries: 0, records: {} } ...
    minimal failing steps: [Publish { publisher: 0, pocket: 0, key: 0, rank: 0 }, Pull { consumer: 0, limit: 0 }, Advance(100), Tick, PublisherJoins(0), Pull { consumer: 1, limit: 1 }, Ack { consumer: 1, lease: Own, key: 0 }]
```

The output above is abridged: the walk's model dump is cut to pocket `a`.

- **The walk:** the shrunk walk is a lapse, a claim and an ack. The model resets the
  count and the code does not.
- **`the_walk_reaches_every_state_and_every_outcome`** fails at the same assertion,
  because it drives the same model.
- **S063 and S064:** S063 (now emptied by nacks) and S064 (renamed) passed, as expected.
- **S024 and S045:** their scenario labels changed and nothing else.
- **`fixture.rs`:** untouched. S077 uses the existing `Fixture::refused`.

### Step 6: the code, and shown to bite

At `2158750`:

- `Pockets::reset_expiries` is `pub(super)` in `pockets.rs`.
- `Queue::ack` calls it once `settle_held(..)?` has returned, so a refused ack returns
  before the call. `nack`, `renew` and the pull never call it.
- The docs on the module, `ack`, `nack` and `PocketView::consecutive_expiries` say so.

`just test` passed 242 of 242.

Each bite was a scratch edit on `2158750`, run with `NEXTEST_PROFILE=ci just test` (exit
100 each) and restored with `git checkout -- <file>`. After each,
`git diff --stat -- crates` printed nothing.

| Bite | Scratch edit | Failing tests |
| --- | --- | --- |
| 1 | reset removed | S077 ("a settling ack", `(Leased, 1)` not 0) and both walk tests |
| 2 | reset moved into `settle_held` | S063 (`(Idle, 1, 0)`, not `(Parked, 3, 0)`), S064 ("after the nack", 0 not 1) and both walk tests |
| 3 | reset in `ack` before the `RecordNotHeld` guard | S077 in `Fixture::refused` ("a refused action changed the queue") and both walk tests |
| 4a | reset added to `Queue::renew` | S064 ("after the renew") and both walk tests |
| 4b | reset added to the pull within a lease | S064 ("after that pull") and both walk tests |
| 5 | the two `turn.reach` calls removed | only `the_walk_reaches_every_state_and_every_outcome`, "never reached": `["ack resets a counted pocket", "nack keeps a counted pocket"]` |

The walk also fails in bites 3, 4a and 4b, beyond the tests the ticket names, because its
model follows the rules. Both new reach labels are reached at the walk's
`max_consecutive_expiries` of 2 with no change to the generator.

The gates at `2158750` all exited 0: `fmt-check`, `clippy`, `test`, `doc`, `effects`,
`metrics` and `coverage`, the last at 99.09% of lines.

### Step 7: the handbook

At `2158750` with uncommitted changes, committed as `b0e5c30`:

- `just check-docs` exited 0.
- The catalog has 77 rows.
- `rg -n -i 'empty batch' docs/reference/http-api.md` exited 0.
- The stale search over `docs` and `crates` exited 1, with no hits.

The text went to the pages Step 7 names, plus `prior-art.md` and `README.md` (see
Overrides). `just check-docs` does not compare the catalog's test names with `crates/`.
Both new names were confirmed with `rg` in `expiry.rs`.

### Step 8: the obligations reconciled

`ids-new.txt` is empty, so no new id needs a tag. Each of the twelve ids whose content
changed is in `ids-after.txt` and tagged under `crates/`. S077's tags are:

- `rule-success.AckSettlesRecord`
- `rule-success.AckDefersNewerVersion`
- `invariant.Pocket.ExpiriesWithinLimit`

No id vanished, so no tag was removed.

### T32-2: the whole diff (step 9)

The reviewers were `codex exec -m gpt-6-astra -s read-only`, one per lens, over
`git diff 53d64b7` at `b0e5c30`, plus an uncommitted rewrap in `prior-art.md`.

Both lenses raised the same finding, conformance C1 and breakage B1 (medium). The ticket's
Verification line
`git diff "$base" -- docs/specs/pachinko.allium | rg -n '^\+.*https?://'` prints the
passing "exit 1" when `git diff` fails, because `rg` returns 1 on empty input. Both
reviewers reproduced this with a bad revision.

- **Verdict:** valid.
- **Fix, in `7d637ac`:** the diff goes to `ai_tmp/T32/spec-diff.txt` with its own exit
  status, and `rg` reads that file.
- **The other Verification pipelines** were checked for the same flaw:
  - the S001 to S077 `diff` fails loudly on empty input;
  - `uniq -d` returns `rg`'s 2 under `pipefail`;
  - the `-B8 ... | rg 'empty pocket'` line must exit 0.

All other questions drew no finding from either lens. Neither reviewer could build in the
read-only sandbox, so their mutation reasoning is from the source; the bites above are the
run evidence. There was no valid high finding, so there was no second round.

### Step 10: `review-change`, the merge and the final check

**`/code-review high`** ran over `53d64b7..7d637ac`, as a read-only Claude agent. It found
no correctness bug and raised seven findings. Each implementer triaged the ones in its own
files:

- **CR1 (low, valid).** The spec's "the decision names the ack only" reads as a ruling on
  nacks. It now says "leave the count as the rules stand: the maintainer decided the ack
  only, and a reset on a nack is neither chosen nor ruled out".
- **CR2 (partly valid).** In `CHANGELOG.md` Unreleased:
  - T22's "keeps eight open questions" (line 36) and T28's "76 scenarios" (line 54) now
    disagree with T32's bullet. They stay as their tickets' record, as the acceptance
    criteria require, and are handed back.
  - T26's "lapses in a row" (line 59) was false, and is fixed.
- **CR3 (valid).** The empty-batch reasons in http-api.md left out deferred records. They
  now read "the pocket has no queued record left (each is held or deferred, or none
  remains)".
- **CR4 (valid).** The revive rules neither create nor require a record. The comment now
  says "creates a record, requires one or answers a publish that leaves one".
- **CR5 (valid, low; handed back).** `config.rs:69`, `types.rs:168`,
  `configuration.md:39` and `terminology.md:36` say "consecutive" without naming what
  resets the count. All four are outside Files touched.
- **CR6 (valid).** Two cases were untested: an ack-emptied pocket parks at
  `max_consecutive_expiries` 1, and idles below the threshold after earlier lapses.
  - Both are now in S063's test, through `Fixture::with_config` and `fork()`, with no new
    id.
  - Two bites, each `NEXTEST_PROFILE=ci just test` exit 100 and then restored:
    - an empty pocket made never to park fails at "limit 1, acked";
    - the reset removed fails at "acked".
- **CR7 (valid, low; handed back).** No test in `crates/pachinko` exercises the HTTP ack
  reset. It holds by construction, since `leases.rs` calls `Queue::ack`, but the fix is a
  service test and lies outside Files touched.

The fixes are in `2681bd6`. After them, at `2681bd6` with uncommitted changes:

- `check-specs`, `analyse-specs`, `check-docs`, `fmt-check`, `clippy`, `metrics` and
  `test` exited 0, with 242 of 242 tests passing;
- obligations stayed at 190 with an empty id diff.

**`review-change`**, over `git diff 53d64b7..HEAD`:

- No public type, dependency, effect or `unsafe` was added.
- The reset follows the two ack rules and runs only after every guard.
- The walk's model resets on the ack branch alone and predicts from the pre-state.

It raised no finding beyond the ones above.

**The merge.** `git fetch origin` exited 0. `origin/wave-2-tickets` is `53d64b7`, the
branch point, and `origin/main` (`611a86e`) is already an ancestor. `git merge
origin/wave-2-tickets` and `git merge origin/main` each printed "Already up to date." and
exited 0, so there is no merge commit and no T33 or T34 lines to reconcile. Merging
`origin/main` stays a no-op until pull request 40 lands.

**The Verification block** was rerun at `2681bd6`. The worktree was clean (`git status
--short` printed nothing) and the merge was a no-op, so `2681bd6` is the merge point. It
ran with `base=53d64b7` and line 528 in its fixed form. This is the abridged output, each
line with its exit status:

```text
just check-specs                                  exit 0
just analyse-specs                                exit 0
just plan-spec pachinko > plan-after.json         exit 0
jq '.obligations | length' before after           190 / 190, exit 0
jq '.diagnostics | length' plan-after.json        0, exit 0
wc -l ids-before.txt ids-after.txt                190 / 190
diff ids-before.txt ids-after.txt                 (nothing), exit 0
wc -l < ids-new.txt                               0; the UNTAGGED loop prints nothing
rg -n 'scenario: S077' -A6 expiry.rs              exit 0
rg -c '^open question '                           10, exit 0
rg -n 'T35' pachinko.allium                       lines 916, 917, exit 0
positive control rg -c 'scenario: S[0-9]+'        expiry.rs:20 lease.rs:12 publish.rs:12 ..., exit 0
scenario set vs S001..S077                        (nothing), exit 0
duplicate scenario tags (uniq -d)                 (nothing), exit 0
catalog rows                                      77, exit 0
git diff "$base" -- spec http-api                 exit 0
git diff "$base" -- spec > spec-diff.txt          exit 0 (must be 0)
rg URL in spec-diff.txt                           exit 1 (1 when the module gains no URL)
-B8 PullWithinLease | rg 'empty pocket'           line 383, exit 0 (must be 0)
rg -i 'empty batch' http-api.md                   line 71, exit 0 (must be 0)
stale search on 53d64b7 (positive control)        7 hits (expiry.rs:400, prior-art.md:229,
                                                  specifications.md:56, work-with-the-specs.md:64
                                                  and :75, testing.md:171 and :244), exit 0
stale search on the branch                        (nothing), exit 1
just fmt-check / clippy / test / doc              exit 0 each
just effects / metrics / coverage                 exit 0 each; coverage 99.09% of lines
just check-docs                                   exit 0, "Validated 47 pages and 48 canonical topics."
just check                                        exit 0
git status --short                                (nothing), exit 0
```

`just check`'s last line:

```text
All checks passed and the worktree is unchanged.
```

The final commit changes only this ticket and T32's row in `tickets/README.md`.

## Open points

- The nack reading is unconfirmed: the maintainer's decision names the ack, so a nack,
  like a renew or a pull, leaves `consecutive_expiries`, and S063 and S064 pin that
  reading. A nack is also a consumer's progress; if the maintainer wants it to reset the
  count, that is a specification change to the two nack rules and those two tests.
  Owner: maintainer.
- Open questions 10 and 11 (Q1, Q2 above) stay open; T35 decides them.
- "A queued pocket has a record" holds by reachability, which the `PullWithinLease`
  comment states; an invariant such as `QueuedHasRecords` would make it checked and add
  obligations. Owner: maintainer.
- No scenario pins a within-lease pull on a pocket emptied by acks or nacks; S036 and
  S067's pull half pin the extension on an empty batch of a pocket that still holds
  records. Owner: maintainer, with T35 or a later ticket.
- `.agents/skills/spec-change/SKILL.md` step 1 still calls `docs/specs/pachinko.allium`
  "the placeholder", "which declares no behaviour"; T31's handbook search did not cover
  `.agents/`. A skill edit takes `just check-agents` and is outside this ticket. Owner:
  maintainer.
- Handed back by T32-1 (B3's imprecision in older text): the comment above
  `PublishNewKey` (`docs/specs/pachinko.allium` line 278) says "Every publish into an
  existing pocket emits PocketTouched". A publish at the stored rank matches no rule and
  emits nothing. Its fix is "Every accepted publish". Owner: T35, which edits that area.
- Handed back by `code-review` CR2: `CHANGELOG.md`'s libpachinko Unreleased section now
  has T22's "keeps eight open questions" (line 36) and T28's "76 scenarios" (line 54)
  beside T32's "ten in all". The acceptance criteria keep them as those tickets' record.
  Fold the unreleased entries into one statement at release. Owner: maintainer, at
  release.
- Handed back by `code-review` CR5: four places say "consecutive" without naming what
  resets the count. They are outside Files touched.
  - Proposed text for the first three: "how many lease expiries, with no ack, release or
    revive between them, park a pocket".
    - `crates/libpachinko/src/config.rs:69`
    - `docs/reference/configuration.md:39`, the page that owns the flag
    - `docs/project/terminology.md:36`
  - `crates/libpachinko/src/types.rs:168`: the same looseness.

  The spec now defines consecutive this way, so none of the four is false. Owner:
  maintainer, or a follow-up ticket.
- Handed back by `code-review` CR7: no test in `crates/pachinko` shows that an ack over
  HTTP resets the count and a nack does not, which the pachinko `CHANGELOG.md` bullet
  claims. It holds by construction, since the service's lease routes call `Queue::ack`
  and `Queue::nack`. The fix is an HTTP test with `--max-consecutive-expiries 2` and
  lapses around an ack. Owner: maintainer, or a follow-up ticket.
- Found by the handbook pass: `docs/reference/http-api.md` lines 93 to 95 say an action
  already waiting at the lapse gets `lease_not_active`. That is wrong for a pull, which
  names no lease: a pull applied after the lapse claims a pocket afresh or answers
  `no_pocket`. This is older text, outside T32's own lines. Owner: maintainer.
