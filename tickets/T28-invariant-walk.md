---
id: T28
title: "Invariant walk and the conformance catalog"
status: todo
depends_on: [T26, T27]
parallel_with: [T30]
branch: ticket/t28-invariant-walk
estimated_size: M
---

# T28: Invariant walk and the conformance catalog

## Context

T23 to T27 built the library and the service rule by rule, each ticket pinning the rules,
invariants, guarantees and scenarios it owns with a test named after the clause it
proves. What that leaves unproven is the rules together: that no sequence of triggers,
refused ones included, can drive the queue into a state the module forbids, and that
every outcome the queue gives is the one the rules' guards predict from the state before
the step. This ticket adds that evidence, a proptest walk over every trigger, and the
page that lets a reader go from a scenario id to the test that pins it. It owns no rule,
invariant, guarantee or scenario: the walk re-checks all 25 invariants after every step,
which is verification, and the ownership ledger in T23, T25, T26 and T27 is untouched.

The decisions this ticket rests on are in the decision table in T22's Context: the
`MemoryRecords` store behind the `PocketRecords` trait, the `LeaseIds` trait with its
counting fake, the `Clock` trait with `FakeClock` and the queue's monotonic clamp, open
question 8 closed with branch A (a late action succeeds: a pull, ack, nack or renew
extends the lease and a release ends it), and open questions 1 and 9 pinned as labelled
placeholders.

The walk is PR 39's, adapted: every step's outcome is predicted from the previous
snapshot by following each rule's guards, and after every step every invariant and every
declared transition is checked. PR 38's machine is the other shape, checks without
prediction, and this ticket takes two things from it: the test layout, one `queue` target
with a `#[cfg(test)]` module per family sharing one fixture, and the header that names
the guards no direct test can make fail. The prototypes this ticket reads are heads of
unmerged branches; a fresh worktree needs `git fetch origin` (network, authorised by
step 1) before `git show` can read them. Never check a prototype branch out into the
ticket's worktree; read with `git show <head>:<path>`.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 38, the Flight service prototype | `tickets/t20-pachinko-prototype-complete` | `cb77f07` | `git show cb77f07:<path>` |
| PR 39, the HTTP service prototype | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` | `git show cf1a6b2:<path>` |

The files to read, each cited where a step uses it: PR 39
`crates/libpachinko/tests/invariants.rs` (the step generator and `publish` from line 219,
`lapse` line 441, `check_invariants` line 608, `check_transitions` line 750, the walk test
`every_invariant_holds_after_every_step_of_any_walk` line 825, the reachability test
`the_walk_reaches_every_state_and_every_outcome` line 877, the entity read-back
`every_pocket_and_record_field_reads_back` line 1051); PR 38
`crates/libpachinko/tests/queue/invariants.rs` (the header naming the six guards,
`check_transitions` line 366, `check_ended_stay_ended` line 429) and
`crates/libpachinko/tests/queue/main.rs` (the layout); PR 38
`crates/libpachinko/Cargo.toml` line 58 (the `queue` test target behind `testing`); PR 39
root `Cargo.toml` lines 13 to 15 (the proptest decision-0007 note) and
`crates/libpachinko/Cargo.toml` line 41 (the development dependency).

### The scenario catalog

The 76 scenarios, as the ownership ledger lists them: the owner is the ticket whose
library-level test pins the scenario; a `wire` scenario is additionally exercised over
HTTP by the service ticket in the Wire column, which is not ownership. Tags: `core`, the
scenario a reader would expect; `edge`, a boundary the rules decide; `wire`, exercised
over HTTP too; `oq:N`, a labelled placeholder for open question N. The executing agent
carries this table into `docs/reference/testing.md` with the owning test names filled in.

| Id | Description | Tags | Owner | Wire |
| --- | --- | --- | --- | --- |
| S001 | Default `lease_ttl` and `max_pull_batch` apply | core | T23 | |
| S002 | Default parking threshold and decay TTLs apply | core | T23 | |
| S003 | `max_consecutive_expiries = 0` is refused | edge | T23 | |
| S004 | `max_pull_batch = 0` is refused | edge | T23 | |
| S005 | Negative TTLs are refused | edge | T23 | |
| S006 | A zero `lease_ttl` lapses at the instant of grant | edge | T26 | |
| S007 | Zero superseded and dead-letter TTLs decay at once | edge | T26 | |
| S008 | `max_pull_batch = 1` caps every pull at one record | edge | T23 | |
| S009 | A publisher joins once | core | T23 | |
| S010 | A consumer joins once | core | T23 | |
| S011 | Publish into a new pocket creates a queued pocket and record | core | T23 | |
| S012 | Publish of a new key appends to an existing pocket | core | T23 | |
| S013 | The same record key in two pockets is two records | edge | T23 | |
| S014 | Supersede replaces rank and payload in place | core | T23 | |
| S015 | Repeated supersede keeps the first slot and sheds every loser | edge | T23 | |
| S016 | A stale publish is shed, not applied and not an error | core | T23 | |
| S017 | An equal-rank publish is rejected (OQ1 placeholder, labelled) | oq:1 | T23 | |
| S018 | Negative and zero ranks order like integers | edge | T23 | |
| S019 | Large ranks keep full precision across supersede and stale | wire | T23 | T24 |
| S020 | Payload bytes and keys round-trip unchanged | wire | T23 | T24 |
| S021 | Publishing into a leased pocket neither requeues it nor escapes the lease | core | T23 | |
| S022 | Publishing into a queued pocket does not move it in the pocket order | edge | T23 | |
| S023 | A publish revives an idle pocket at the back | core | T25 | |
| S024 | A supersede or a stale publish revives a parked pocket | edge | T26 | |
| S025 | An equal-rank publish does not revive a parked pocket | oq:1 | T26 | |
| S026 | Only a publish revives a parked pocket (OQ7 as the rules stand) | oq:7 | T26 | |
| S027 | A pull claims the front pocket, leases it and holds the batch | core | T23 | |
| S028 | A positive limit returns at most that many, oldest first, and later pulls continue | core, wire | T23 | T24 |
| S029 | A limit above `max_pull_batch` is capped | edge | T23 | |
| S030 | A limit above the queued count returns every queued record | edge | T23 | |
| S031 | A zero-limit pull claims a pocket and returns an empty batch | edge, wire | T23 | T24 |
| S032 | A negative limit behaves like zero | edge | T23 | |
| S033 | A pull with no queued pocket returns `NoPocketAvailable`, not an error | core, wire | T23 | T24 |
| S034 | Idle, parked and leased pockets are never claimed | edge | T26 | |
| S035 | A consumer with a lease never gets `NoPocketAvailable` or another pocket | edge | T25 | |
| S036 | A pull under a lease extends the lease | core | T23 | |
| S037 | A held record is not handed out again under the same lease | core | T23 | |
| S038 | Pockets are claimed in the order they entered | core | T23 | |
| S039 | A second consumer never receives records from a leased pocket | core | T23 | |
| S040 | An ack settles a held record | core, wire | T25 | T25 |
| S041 | An ack extends the lease | core | T25 | |
| S042 | Rejected acks change nothing, not even the lease's expiry | edge | T25 | |
| S043 | A supersede while the key is held defers it on ack | edge | T25 | |
| S044 | A deferred record restarts at the back of its pocket | edge | T25 | |
| S045 | Two supersedes while held: both losers are shed and the held version is untouched | edge | T25 | |
| S046 | A stale check while held compares with the latest rank, not the held one | edge, oq:1 | T25 | |
| S047 | A nack dead-letters the held version on the first nack, without redelivery | core, wire | T25 | T25 |
| S048 | A supersede while the key is held defers it on nack, and the lapse frees it | edge | T26 | |
| S049 | Rejected nacks change nothing and create no dead letter | edge | T25 | |
| S050 | A renew extends the lease to now plus `lease_ttl` | core, wire | T25 | T25 |
| S051 | Renewals do not stack | edge | T25 | |
| S052 | A renew of an ended lease is rejected and moves nothing | edge | T25 | |
| S053 | A release requeues the pocket at the back and its held records in place | core, wire | T25 | T25 |
| S054 | A release with only deferred records requeues the pocket rather than idling it | edge | T25 | |
| S055 | A release idles an empty pocket | core | T25 | |
| S056 | A release resets the consecutive-expiry count | edge | T26 | |
| S057 | A second release is rejected; a release at the grant instant is legal | edge | T25 | |
| S058 | A released lease never lapses afterwards | edge | T26 | |
| S059 | A lease lapses exactly at `expires_at` | core | T26 | |
| S060 | A lapse keeps the pocket at the front and requeues its records | core | T26 | |
| S061 | A lapse with no records idles the pocket | edge | T26 | |
| S062 | Three consecutive lapses park the pocket, and a publish revives it | edge | T26 | |
| S063 | At the threshold an empty pocket parks rather than idling | edge | T26 | |
| S064 | Progress under a lease does not reset the expiry count | edge | T26 | |
| S065 | A lapse fires once per lease | edge | T26 | |
| S066 | A renew or release after `expires_at` but before the lapse is processed succeeds (branch A) | oq:8 | T26 | |
| S067 | An ack or pull after `expires_at` but before the lapse is processed succeeds (branch A) | oq:8 | T26 | |
| S068 | A consumer that re-claims the same pocket cannot use its old lease | edge, wire | T25 | T25 |
| S069 | After a lapse, the old lease is fenced even when the key has a newer rank | edge | T26 | |
| S070 | A consumer cannot act on another consumer's lease | edge | T25 | |
| S071 | A key acked, then published again within the same lease, is a new queued record (OQ9 as the rules stand, labelled) | oq:9 | T25 | |
| S072 | A key nacked, then published again within the same lease, is a new queued record (OQ9 as the rules stand, labelled) | oq:9 | T25 | |
| S073 | Superseded entries decay at `shed_at` plus `superseded_ttl` | core | T26 | |
| S074 | Dead letters decay at `nacked_at` plus `dead_letter_ttl` | core | T26 | |
| S075 | Pockets and records form one strict, deterministic order | edge | T26 | |
| S076 | The consumer view exposes only the active lease and its held versions | core | T25 | |

Counts: T23 owns 31, T25 owns 23, T26 owns 22; 76 in all. Wire exercises: T24 five
(S019, S020, S028, S031, S033) and T25 five (S040, S047, S050, S053, S068).

## Goal

`just test` runs a seeded proptest walk over `Queue<MemoryRecords, FakeClock,
CountingLeaseIds>` that predicts each step's outcome from the rules' guards, checks all
25 invariants and every declared transition after every step, and reads its limits from
the queue; a reachability test proves the generator reaches every state and every outcome
the module names; `docs/reference/testing.md` carries the 76-scenario catalog with the
owning test of each by file and name; every obligation `just plan-spec pachinko` prints
is tagged on a test and the hand-back maps each id to its test; coverage stays at or above
90%; `just check` is green.

## Non-goals

- No new rule, no change to `docs/specs/pachinko.allium`, and no ownership moved: the
  scenarios and obligations stay with T23, T25, T26 and T27. An obligation no test
  carries gets a test here only where the walk's snapshot already shows it; otherwise it
  is handed back to its owner.
- No walk over HTTP. The service's wire tests are T24's, T25's, T27's and T29's.
- No `proptest-regressions/` file. The runner is seeded and persists nothing, so no
  recipe writes into the worktree; a failure is reproduced from the seed and the shrunk
  case the output prints.
- No mutation run and no benchmark; T30 measures.
- No change to `pyproject.toml`'s recipe list.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | shared, own lines | `[workspace.dependencies] proptest` with the decision-0007 note, `default-features = false`, `features = ["std"]` |
| `Cargo.lock` | shared, own lines | Regenerated by `just lock`, never edited by hand: proptest's packages |
| `fuzz/Cargo.lock` | shared, own lines | Committed only if `just lock` moved it |
| `crates/libpachinko/Cargo.toml` | shared, own lines | `proptest = { workspace = true }` under `[dev-dependencies]`; the `queue` target unchanged |
| `crates/libpachinko/src/queue.rs` | shared, own lines | A `config(&self) -> &Config` accessor if T23 did not land one, and any `Snapshot` field the checks need, behind `testing` |
| `crates/libpachinko/tests/queue/main.rs` | shared, own lines | `#[cfg(test)] mod invariants;` and, if split, `mod walk;` |
| `crates/libpachinko/tests/queue/invariants.rs` | new | The checks, the walk test and the reachability test |
| `crates/libpachinko/tests/queue/walk.rs` | new | The step generator and the per-step predictions, if `invariants.rs` would otherwise pass the metrics gate's thousand lines |
| `crates/libpachinko/tests/queue/fixture.rs` | shared, own lines | Only a helper the walk needs that the fixture lacks |
| `crates/libpachinko/tests/queue/identity.rs`, `publish.rs`, `pull.rs`, `settle.rs`, `lease.rs`, `expiry.rs`, `decay.rs`; `crates/libpachinko/src/config.rs` | shared, own lines | A `// scenario:` tag only, where step 6 finds one missing, duplicated or on the wrong test (`config.rs` for S005's doctest); a test that is wrong is handed back to its owner |
| `crates/pachinko/tests/http.rs` | shared, own lines | A `// scenario:` tag only, where step 6 finds one of the ten `wire` ids missing, duplicated or on the wrong test; a wire test that is wrong is handed back to T24 or T25 as the blocking finding step 6 states |
| `docs/reference/testing.md` | shared, own lines | "Layout" (the regression-file paragraph), "Tests from the specifications" (the walk), the new "Scenario catalog" section, "What the current suite proves" |
| `docs/reference/configuration.md` | shared, own lines | The root `Cargo.toml` row's `[workspace.dependencies]` sentence names proptest |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | Where it names the development dependencies (line 132 today) |
| `docs/decisions/0007-dependency-policy.md` | shared, own lines | One sentence under "Harness dependencies": proptest as a library development dependency |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Added: the walk and the catalog |
| `tickets/README.md` | shared, own lines | The index row's status |
| `tickets/T28-invariant-walk.md` | ticket | `status:` and hand-back notes |

T30 edits `Cargo.toml`, `Cargo.lock`, `crates/libpachinko/Cargo.toml`,
`docs/reference/testing.md`, `CHANGELOG.md` and `tickets/README.md` on its own lane, so
each is `shared, own lines` and the wave-2 rule applies (`CONVENTIONS.md` §11):
`origin/main` merged before the push, `Cargo.lock` regenerated and never merged by hand,
verification rerun and requoted.

## Steps

1. Create the worktree from `main` after T26 and T27 have merged (`README.md`, "How to
   pick up a ticket"), or, in a clean worktree already initialized on `main` whose branch
   carries nothing, create the branch there and skip `just initialize`. A new worktree
   runs `just initialize` (network, authorised by this step). Run `git fetch origin`
   (network, authorised by this step) so that `git show cb77f07:<path>` and
   `git show cf1a6b2:<path>` can read the prototypes. Run `just check` on the untouched
   worktree and quote its last line as the baseline. Use the `plan-change` skill for this
   step and the next: read this ticket, the decision table in T22's Context, the ledger
   sentences in T23's, T25's, T26's and T27's Goals, and `tickets/CONVENTIONS.md` §11
   and §14.
2. **The dependency.** Use the `rust-change` skill. Add proptest to
   `[workspace.dependencies]` in the root `Cargo.toml` with the decision-0007 note in the
   form the file uses for divan (PR 39 root `Cargo.toml` lines 13 to 15 is the wording:
   proptest drives the shrinking state-machine walk over the queue's invariants, as a
   development dependency only; std has no generator or shrinker), `default-features =
   false` and `features = ["std"]`, and a full `x.y.z` caret version; PR 39's pin at
   `cf1a6b2` was 1.12.0, re-pinned to the current release on the day. `crates/libpachinko/Cargo.toml`
   takes `proptest = { workspace = true }` under `[dev-dependencies]`. Run `just lock`
   (network, authorised by this step) and commit the regenerated `Cargo.lock`, and
   `fuzz/Cargo.lock` if it moved. Confirm `just deps-unused`, `just deny` and
   `just features` are green with proptest present: it is development-only, so it is
   outside cargo-deny's graph (`deny.toml`'s `exclude-dev`), and `just features` builds
   nothing new. Quote the three.
3. **The walk.** Use the `rust-change` skill. Write `crates/libpachinko/tests/queue/invariants.rs`
   (and `walk.rs` if the metrics gate needs the split) as a `#[cfg(test)]` module of the
   `queue` target, from PR 39 `crates/libpachinko/tests/invariants.rs`, with these
   properties, each of which the adversarial review in step 7 asks about:
   - **Generic over the three effects.** The walk's functions take
     `&mut Queue<R, C, L>` for any `R: PocketRecords`, `C: Clock`, `L: LeaseIds` and a
     closure that sets the clock, so a later store (`LogRecords` is 0019's reopen option)
     runs the same walk unchanged; one `#[test]` instantiates it over `MemoryRecords`,
     `FakeClock` and `CountingLeaseIds`.
   - **Small domains, every trigger.** Two publishers, three consumers, three pockets,
     three keys, ranks 0 to 3, limits -1 to 3; steps `PublisherJoins`, `ConsumerJoins`,
     `Publish`, `Pull`, `Ack`, `Nack`, `Renew`, `Release`, `Advance` (forwards up to
     `lease_ttl` and a bit beyond, and backwards, which the queue's `now` must ignore) and
     `Tick`. A lease an action names is the consumer's own, any lease handed out so far
     (ended ones and other consumers' included), or one never issued, as PR 39's
     `LeaseChoice` does. Walks of 1 to 80 steps, 512 cases, a deterministic `ChaCha`
     runner, `failure_persistence: None`.
   - **Every outcome predicted from the pre-state.** Before each step the walk takes the
     queue's `Snapshot` and derives the expected result from the rules' guards, as
     `publish` (line 219) and `lapse` (line 441) of PR 39
     `crates/libpachinko/tests/invariants.rs` do: `Created`, `NewKey`, `Superseded`,
     `Stale` or the `EqualRank` error (the open question 1 placeholder, by name);
     `UnknownPublisher` and `UnknownConsumer` for a party that never joined; a batch of
     the queued records with the smallest positions, at most
     `min(limit, max_pull_batch)` and none for a limit that is not positive, under the
     consumer's own lease or a fresh one on the front pocket, else `NoPocketAvailable`;
     for ack, nack, renew and release the refusals in rule order, `LeaseNotActive` then
     `LeaseNotYours` then `RecordNotHeld`, else the settle outcome (settled when the held
     rank is the latest, deferred when not) or the release fate (queued with records,
     the pocket at a position equal to the pre-state's `next_position` and
     `next_position` advanced by one, as `ReleaseRequeuesPocket` assigns them; idle
     without); and `expires_at` equal to the queue's `now` plus the queue's `lease_ttl`
     after every extending action. **Branch A** is predicted, not tolerated:
     an action naming an active lease whose `expires_at` is at or before `now`, with no
     tick between, succeeds, extending the lease unless it is a release, which ends it,
     and the walk labels it so the reachability test can demand it.
   - **The tick predicted from the pre-state.** T26's `tick()` replaces PR 39's `lapse`
     and `decay` steps. The prediction lists the active leases with `expires_at <= now`
     in `(expires_at, lease id)` order, gives each the fate the three expiry rules decide
     (parked at the threshold, idle with no records below it, else queued and kept at
     the front with its position untouched), counts the superseded records and dead
     letters with `expires_at <= now`, and compares all of it with the `TickReport`.
   - **All 25 invariants after every step, by name.** The entity invariants
     `LeasedHasOneActiveLease`, `UnleasedHasNoActiveLease`, `IdleHasNoRecords`,
     `ExpiriesWithinLimit`, `HeldVersionNotNewer`, `ShedBeforeExpiry`,
     `NackedBeforeExpiry`, `GrantedBeforeExpiry`, `EndedAfterGrant`; the module
     invariants `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`,
     `OneRecordPerKeyPerPocket`, `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`,
     `OutstandingOnlyWhileLeased`, `UniquePocketPosition`, `UniqueRecordPosition`,
     `PocketPositionsBelowNext`, `RecordPositionsBelowNext`; and the five over config,
     `ExpiryLimitPositive`, `PullBatchPositive`, `LeaseTtlNonNegative`,
     `SupersededTtlNonNegative`, `DeadLetterTtlNonNegative`. Each assertion's message is
     the invariant's name. The three TTL invariants hold by `Duration`'s type, and the
     walk witnesses them with checks that can fail: every `SupersededRecord` has
     `expires_at` equal to `shed_at` plus the queue's `superseded_ttl`, every
     `DeadLetter` has `expires_at` equal to `nacked_at` plus `dead_letter_ttl`, and
     every extension lands on `now` plus `lease_ttl`.
   - **Config read from the queue, never from the test's own helper.** `max_pull_batch`,
     `max_consecutive_expiries` and the three TTLs come from the queue under test
     (`queue.config()`, added behind `testing` in `crates/libpachinko/src/queue.rs` if T23
     did not land it); the walk's `config()` helper is used once, to build the queue. PR
     39 `check_invariants` (line 608) reads its own constants; that is the defect this
     property removes.
   - **Uniqueness over a `Vec`.** The `Snapshot`'s publishers, consumers, pockets,
     records and leases are `Vec`s, and each uniqueness check collects the keys into a
     `Vec` and compares its length with the deduplicated set's, as PR 39's `distinct`
     does; a set-typed snapshot would hide the duplicate the invariant forbids.
   - **Every declared transition after every step.** Pocket: `queued -> leased`,
     `leased -> queued`, `leased -> idle`, `leased -> parked`, `idle -> queued`,
     `parked -> queued`; record: `queued -> held`, `held -> queued`, `held -> deferred`,
     `deferred -> queued`; lease: `active -> released`, `active -> expired`, with
     `released` and `expired` terminal, so an ended lease's view never changes again and
     never shows as active (PR 38 `check_ended_stay_ended`, line 429). A pocket is
     created queued and never removed; a record is created queued and leaves only by the
     settle of its held version; a lease is created active; a held version does not
     change while held (PR 39 `check_transitions`, line 750).
   - **The guarantees against the previous snapshot.** `OldestFirst`, `ExclusivePocket`
     and `OneVisitPerKeyPerLease` for every batch, as PR 39 `check_batch` does, and the
     pocket order equal to the queued pockets sorted by position.
   - **The header.** The file's doc comment names, as PR 38's does, the six guards no
     direct test can make fail because an invariant the walk checks implies each:
     `rule-failure.PullWithinLease.2`, `rule-failure.ReleaseRequeuesPocket.3`,
     `rule-failure.ReleaseIdlesEmptyPocket.3`, `rule-failure.LeaseExpiryParksPocket.3`,
     `rule-failure.LeaseExpiryKeepsPocketAtFront.3` and
     `rule-failure.LeaseExpiryIdlesEmptyPocket.3`, each requiring a lease's pocket to be
     leased, which `LeasedHasOneActiveLease` and `UnleasedHasNoActiveLease` make true of
     every active lease. The walk test carries an `// obligation:` line for each of the
     25 `invariant.*` ids, the three `transition-rejected.*` ids,
     `transition-terminal.Lease.status` and those six.

   The walk test is `every_invariant_holds_after_every_step_of_any_walk`. Run
   `just test` and quote the `queue` target's lines.

   **Adversarial review** (`CONVENTIONS.md` §14): the walk, uncommitted, against the
   properties above, the catalog in Context and `docs/specs/pachinko.allium`, before step
   4's reachability test and step 5's breaks build on it. Ask: whether any prediction
   reads the queue's answer to the step rather than deriving it from the pre-state
   `Snapshot` and the rules' guards; whether the ack, nack, renew and release refusals
   follow the rules' `requires` order; whether branch A is predicted as success and would
   fail under the rejected reading; whether the tick prediction orders due leases by
   `(expires_at, lease id)` and gives each the fate the three expiry rules decide;
   whether any limit or TTL comes from the test's helper rather than from the queue;
   whether a uniqueness check runs over a set; whether a transition the module does not
   declare would pass the edge check; whether the strategy can reach, within 80 steps,
   every step kind, a lease past `expires_at` with no tick between, a parked pocket and a
   dead letter; and whether the walk is generic over `R`, `C` and `L` as stated. Two
   reviewer subagents with fresh contexts run in parallel, one lens each, `conformance`
   and `breakage`, each running `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T28-1/findings-<lens>.md "<brief>" < /dev/null` (§14's command; `-o`
   writes only the reviewer's last message, and a read-only reviewer writes no file of
   its own); separate triage subagents check every finding against the tree and write
   `ai_tmp/review/T28-1/verdicts.md`, each finding valid with its fix or invalid with the
   reason; one writer applies the valid fixes one at a time, only within Files touched,
   and hands back any fix outside it; if a high-severity finding was valid, a second
   round reviews the fix diff and writes beside the first with a `-2` suffix; two rounds
   at most.
4. **The reachability test.** `the_walk_reaches_every_state_and_every_outcome` draws 512
   walks from the same seeded generator and asserts that every label in a `WANTED` list
   was reached, naming the missing ones, as PR 39 line 877 does. The list covers at
   least: both join outcomes for a publisher and for a consumer; `Created`, `NewKey`,
   `Superseded`, `Stale`, `EqualRank` and `UnknownPublisher` on publish; a revive from
   idle and from parked; `NoPocketAvailable`, a claiming pull, a pull within a lease, an
   empty batch and a batch of several; settled and deferred on ack and on nack;
   `LeaseNotActive`, `LeaseNotYours` and `RecordNotHeld` on an action; a renew and a
   release accepted and refused; a release to queued and to idle; a lapse to queued, to
   idle and to parked; a decayed superseded record and a decayed dead letter; every
   pocket state; a queued record in a queued, a leased and a parked pocket, a held and a
   deferred record in a leased pocket; a branch-A action after `expires_at` and before
   the tick; and a renew and a release with the clock behind the queue's `now`. A label
   nothing produces is a defect in the generator's weights, fixed there.
5. **Shown to bite.** The testing page's convention: break the code on purpose and see
   the named test fail. On scratch copies, never committed, make at least four breaks
   and quote the failing assertion of each: `Supersede`'s `rank > record.rank` guard
   loosened to `>=` (the walk's publish prediction fails on `EqualRank`); the parking
   threshold's `>=` tightened to `>` (the lapse prediction fails on `Parked`); an ended
   lease left in the active registry (`OneActiveLeasePerConsumer` or the `LeaseNotActive`
   prediction fails); a release that keeps the pocket's old position (the release
   prediction fails on the position and on `next_position`; the pocket-order check alone
   would pass it, because the index still sorts by the kept position). Restore each
   before the next.
6. **The catalog and the obligations.** Use the `review-docs` skill.
   - Add a "Scenario catalog" section to `docs/reference/testing.md`, after "Tests from
     the specifications": one table, columns `Id`, `Description`, `Tags`, `Owning test`,
     `Wire test`, one row per scenario in the catalog in Context, the owning test as
     the file and the function name (`crates/libpachinko/tests/queue/publish.rs`,
     `a_stale_publish_is_shed_not_applied`, in that form) found by
     `rg -n 'scenario: S0NN'`, and the wire test the same way under
     `crates/pachinko/tests/`. Every id must be tagged on exactly one test under
     `crates/libpachinko/tests/` or, for S005, on the `compile_fail` doctest T23 wrote
     under `crates/libpachinko/src/`, whose catalog row names the file and the item
     whose doctest carries the tag; each of the ten `wire` ids on exactly one test
     under `crates/pachinko/tests/` as well. A scenario tagged on a test that does not
     exercise it, or tagged twice, or not at all, is fixed in the owning test file if
     the fix is a tag (the scenario files and `crates/pachinko/tests/http.rs` in Files
     touched; T23, T24, T25 and T26 have merged, so they are shared files, not another
     lane's) and handed back to the owning ticket if it is a test, a wire test to T24
     or T25. A hand-back is a blocking finding for the wave,
     never an Open point: the owning ticket reopens and lands the test on its own lane,
     and T31 is not done until it has merged and the counts below are rerun.
   - Reconcile the obligations: run `just plan-spec pachinko` (190 at `611a86e`; a
     different count is reconciled in the hand-back, never hidden) and
     `rg -No 'obligation: [^ ]+' crates/libpachinko/tests crates/libpachinko/src`.
     Without a script in the repository, write in the hand-back one line per obligation
     id naming the test that carries it, and the six the walk's header carries. An id no
     test carries after T23 to T27 gets a test in `invariants.rs` only where the walk's
     snapshot already shows it, as PR 39's `every_pocket_and_record_field_reads_back`
     (line 1051) did for `entity-fields.*` and `projection.*` ids; otherwise it is handed
     back to the owning ticket, named, and blocks T31 the same way.
   - "Tests from the specifications" says what the walk proves and how to read a
     failure (the seed and the shrunk case in the output); "Layout" says the property
     test persists no regression file and why; "What the current suite proves" gains the
     walk's and the reachability test's rows. `docs/reference/configuration.md`,
     `docs/how-to/maintain-dependencies.md` and decision 0007 name proptest where they
     name divan. `CHANGELOG.md`: libpachinko Unreleased, Added. Run `just check-docs`.
7. **Adversarial review** (`CONVENTIONS.md` §14): the whole diff against this ticket's
   Goal, Files touched and Acceptance criteria, the properties listed in step 3, the
   catalog in Context and `docs/specs/pachinko.allium`. Ask: whether any walk check
   cannot fail (an assertion on a constant, a prediction that copies the queue's answer,
   a `Duration` compared with zero); whether a scenario is tagged on a test that does not
   exercise it; whether any limit or TTL is read from the test's own helper rather than
   from the queue; whether a reachability label is one nothing produces, or one so weak
   that a generator missing a state would still pass; whether the branch-A prediction
   would also pass under the rejected reading; whether a transition the module does not
   declare could pass the edge check; whether an obligation id is tagged on a test that
   does not prove it; and whether coverage is counted on test code. Two reviewer
   subagents with fresh contexts run in parallel, one lens each, `conformance` and
   `breakage`, each running `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T28-2/findings-<lens>.md "<brief>" < /dev/null` (§14's command; `-o`
   writes only the reviewer's last message, and a read-only reviewer writes no file of
   its own); separate triage subagents check every finding against the tree and write
   `ai_tmp/review/T28-2/verdicts.md`, each finding valid with its fix or invalid with the
   reason; one writer applies the valid fixes one at a time, only within Files touched,
   and hands back any fix outside it; if a high-severity finding was valid, a second
   round reviews the fix diff and writes beside the first with a `-2` suffix; two rounds
   at most. Then `just coverage` (at or above 90% of lines over `crates/*/src/**`) and
   `just check`, and commit on the ticket branch with the hand-back notes quoting each
   output headed by the commit it ran at. Before the push: `git fetch origin` (network,
   authorised by this step), merge `origin/main` if T30 landed first, regenerate
   `Cargo.lock` with `just lock` (network, authorised by this step) if the merge touched
   it, rerun the Verification block and requote it. **Authorisation required: push and
   open a pull request.** Stop and ask.

## Acceptance criteria

- `just lock-check` is green with proptest in `Cargo.toml`, `Cargo.lock` and
  `crates/libpachinko/Cargo.toml`; `just deps-unused`, `just deny` and `just features`
  are quoted green.
- `just test` runs `every_invariant_holds_after_every_step_of_any_walk` (512 seeded
  cases) and `the_walk_reaches_every_state_and_every_outcome` in the `queue` target, both
  green and quoted.
- The walk's assertions name all 25 invariants, the twelve declared edges and the two
  terminal lease states, and read `max_pull_batch`, `max_consecutive_expiries` and the
  three TTLs from the queue under test.
- Every step's outcome is predicted from the previous snapshot, branch A and the
  `EqualRank` placeholder included; the hand-back lists at least four breaks that made
  the walk fail, each with the assertion that fired.
- The reachability test's list covers every item step 4 names, and the hand-back shows
  it failing when one label is removed from the generator's reach.
- Each of S001 to S076 is tagged on exactly one test under `crates/libpachinko/tests/`
  or, for S005, on one doctest under `crates/libpachinko/src/`; each of the ten `wire`
  ids on exactly one test under `crates/pachinko/tests/`; and the "Scenario catalog"
  table in `docs/reference/testing.md` names the owning test and the wire test of each
  by file and function, a doctest by its file and item.
- Every obligation id `just plan-spec pachinko` prints is tagged on a test; the hand-back
  maps each id to its test, names the six the walk's header carries, and lists any id
  handed back with its owner.
- `just coverage` reports at or above 90% of lines over `crates/*/src/**`.
- `just check-docs` and `just check` are green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just lock-check
just deps-unused
just deny
just features
just test
set -o pipefail
rg -c 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src; echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort -u | diff - <(printf 'scenario: S0%02d\n' $(seq 1 76)); echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort | uniq -d; echo "exit $?"
rg -c 'scenario: S[0-9]+' crates/pachinko/tests; echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/pachinko/tests | sort -u \
  | diff - <(printf 'scenario: %s\n' S019 S020 S028 S031 S033 S040 S047 S050 S053 S068)
echo "exit $?"
rg -o --no-filename 'scenario: S[0-9]+' crates/pachinko/tests | sort | uniq -d
echo "exit $?"
just plan-spec pachinko
rg -o --no-filename 'obligation: [^ ]+' crates/libpachinko/tests crates/libpachinko/src \
  | sort -u | wc -l
just coverage
just check-docs
just check
git status --short
```

Each `rg -c` line is the positive control for the pipelines after it: run alone, it
exits 0 and prints a count per file (`CONVENTIONS.md` §11, "Checks fail closed"). Under
`pipefail` a pipeline fails when its `rg` matched nothing, so an empty `uniq -d` is a pass
only beside `exit 0`. The pattern takes the whole token after the `scenario:` marker, with
no range in it, so an id outside S001 to S076, or one with an extra digit, reaches the
`diff` and fails it, where a pattern of fixed width would trim the one and drop the other.
The two `diff` lines print nothing and exit 0 when the id set is exactly S001 to S076 and
exactly the ten `wire` ids; the two `uniq -d` lines print nothing when no id is tagged
twice across all the files, which a count per file cannot show. The
library lines search `crates/libpachinko/src` as well because S005's tag is on T23's
`compile_fail` doctest in library source, not under `tests/`. Diagnostics with file and
function come from `rg -n`, as step 6 reads them. The obligation count is compared by hand
with `just plan-spec pachinko`'s, and the mapping is in the hand-back.

## Hand-back notes

### Baseline

At `1fdaab3`, in the fresh lane worktree `wave/t28` after `just initialize` (exit 0),
`just check` (exit 0):

```text
All checks passed and the worktree is unchanged.
```

### Wave overrides

- Worked in the lane worktree `wave/t28` from `1fdaab3` (T22 to T27 and T29 landed), not
  on `ticket/t28-invariant-walk` from `main`; `just initialize` run there once, as the
  lead authorised. `git fetch origin` was run so `cb77f07` and `cf1a6b2` resolve.
- `git diff main...HEAD` read as `git diff 1fdaab3...HEAD` plus the uncommitted change.
- Step 7's fetch and merge of `origin/main`, push and pull request: not run: one pull
  request at T31. The lead merges the lanes into `mvp-inimal-but-sufficient`; the
  Verification block is requoted at that merge commit.
- The §14 reviewers, triage and fix writer were run by the wave lead; this agent stopped
  at T28-1 and T28-2 uncommitted. Step 2's lockfile was therefore not committed on its
  own; it lands in the ticket's one commit.

### Departures from the ticket text

- No `src/queue.rs` change: `Queue::config()` already exists, public and not behind
  `testing`, so the walk reads it as it is.
- S066 and S067 are tagged `decision:0019`, not `oq:8`: decision 0019 closed open
  question 8 in T22 (branch A), so they are no longer placeholders. The walk predicts
  branch A as success.
- Files touched deviation, approved by the lead: `crates/libpachinko/tests/queue/expiry.rs`
  carried S066 and S067 on two tests each (one per half of the scenario). The tag stays on
  `a_renew_after_expires_at_but_before_the_lapse_succeeds` and
  `an_ack_after_expires_at_but_before_the_lapse_succeeds`; the release and pull halves now
  carry a plain comment the pattern does not match, and the catalog row names both tests.
  Comment-only.
- The walk tags five obligations beyond the ticket's six header guards, because no test
  carried them after T23 to T27. The walk's post-state prediction shows four of them:
  `rule-failure.ReviveIdlePocket.1`, `rule-failure.RequeueHeldRecord.1`,
  `rule-failure.RequeueDeferredRecord.1` (existence guards whose rejection is
  unreachable) and `surface-provides.Consuming` (`LeaseNotActive` and `LeaseNotYours`
  predicted over every lease choice). The fifth, `rule-failure.RequeueDeferredRecord.2`,
  is reached on every freed held record, but no test can observe it: the rejection
  leaves the same state as `RequeueHeldRecord`'s effect on the same `Freed`. Its tag
  stays, with the header's caveat. It is not handed back to T25, which is done: the
  wave lead recorded an exception, and it is an Open point for the maintainer. T28-1's
  reviewers did not find the tags dishonest; the header says which is which.
- The walk predicts the whole post-state, not only the outcome: each pre-state `Snapshot`
  is projected into a model (records keyed by key, so the store's order cannot matter),
  the step's rules are applied to it, and the post-state's projection must equal it. The
  clock clamp is modelled at the queue's read points: an accepted step, a publish by a
  joined publisher (`EqualRank` included) and a tick read the clock; a refused lease
  action, a `NoPocketAvailable` pull, a join and an `Advance` do not.

### Step 2: the dependency

At `1fdaab3` plus the uncommitted change: `just lock-check` exit 0
(`✔ Lock-file was already up-to-date`), `just deps-unused` exit 0 (`✓ no issues found`),
`just deny` exit 0 (`bans ok, licenses ok, sources ok`), `just features` exit 0.
`fuzz/Cargo.lock` did not move.

### T28-1: the walk (step 3)

Harness and model: two reviewer lenses, `conformance` and `breakage`, each
`codex exec -m gpt-6-astra -s read-only` (codex-cli 0.166.0, exit 0), with separate triage
and one writer, run by the wave lead; outputs under `ai_tmp/review/T28-1/`. 4 findings,
4 valid, none high after triage, so no second round.

| Id | Verdict | Reason |
| --- | --- | --- |
| C1 | valid, low | `walk.rs` reserved `1 << 64` and `u128::MAX` as sentinels in the lease-id space, which `LeaseIds` does not reserve; no source in the tree reaches them, so latent |
| B1 | valid, low (from medium) | The same defect from the breakage lens |
| C2 | valid, low | The header called `RequeueDeferredRecord.2`'s rejection unreachable; it is reached on every freed held record, but cannot be observed |
| B2 | valid, low | The same header defect |

Applied by the writer: a pending claim is held in `Model.pending` until the queue's
answer names it, and the never-issued lease is the smallest id no pull has handed out,
chosen each step (C1, B1); the header now separates the three unreachable existence
guards from `RequeueDeferredRecord.2` (C2, B2). Handed back: nothing.

Rerun after the fixes, at `1fdaab3` plus the uncommitted change: `just fmt-check`,
`just clippy` and `just metrics` exit 0 (`Quality score: 100.0%` for both crates);
`just lock-check`, `just deps-unused`, `just deny` and `just features` exit 0;
`just test` exit 0:

```text
        PASS [   0.138s] (200/235) libpachinko::queue invariants::every_invariant_holds_after_every_step_of_any_walk
     Summary [   0.625s] 235 tests run: 235 passed, 0 skipped
```

### Steps 4 and 5: reachability and shown to bite

`the_walk_reaches_every_state_and_every_outcome` draws 512 walks from the walk's seeded
generator and requires 61 labels: both join outcomes for each party; every publish
outcome and refusal; a revive from idle and from parked; `NoPocketAvailable`,
`UnknownConsumer`, a claim, a pull within a lease, an empty batch and a batch of several;
settled, deferred and every refusal on ack and nack; a renew and a release accepted and
refused; a release to queued and to idle; a lapse to queued, idle and parked; both
decays; every pocket state and every record state in each pocket state the module
allows; branch A on pull, ack, nack, renew and release; and a renew and a release with the
clock behind the queue's `now`.

The breaks, each on a scratch edit restored before the next, rerun at T28-2 at `1fdaab3`
plus the uncommitted change by `ai_tmp/wave/T28/breaks-exit.sh` (scratch, not committed).
Each runs the `$` command shown; its output is filtered to the first `panicked` line and
the three after it (`grep -m1 -A3 'panicked'`) and to the `test result:` line, and the
command's own exit status is recorded, not the filter's. Breaks 2 to 4 fire on the
whole-state prediction; the `Model` dumps are cut here to their first 200 characters.
After the restore, `git status --short crates/libpachinko/src` prints nothing,
the three edited files match their pre-run checksums, and both commands rerun with exit 0:

```text
=== 1 supersede guard > to >=
$ cargo test -p libpachinko --all-features --locked --test queue every_invariant
thread 'invariants::every_invariant_holds_after_every_step_of_any_walk' (412974285) panicked at crates/libpachinko/tests/queue/walk.rs:864:5:
assertion `left == right` failed: publish
  left: Ok(Superseded)
 right: Err(EqualRank { key: RecordKey("k2"), rank: Rank(3) })
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.01s
exit 101
=== 2 parking threshold >= to >
$ cargo test -p libpachinko --all-features --locked --test queue every_invariant
thread 'invariants::every_invariant_holds_after_every_step_of_any_walk' (413031353) panicked at crates/libpachinko/tests/queue/walk.rs:1110:5:
assertion `left == right` failed: the state the rules predict
  left: Model { next_position: Position(11), publishers: {PublisherId("p0"), PublisherId("p1")}, consumers: {ConsumerId("c0"): None, ConsumerId("c1"): None, ConsumerId("c2"): Some(LeaseId(4))}, pockets: {Pock
 right: Model { next_position: Position(11), publishers: {PublisherId("p0"), PublisherId("p1")}, consumers: {ConsumerId("c0"): None, ConsumerId("c1"): None, ConsumerId("c2"): Some(LeaseId(4))}, pockets: {Pock
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.03s
exit 101
=== 3 ended lease left in the active registry
$ cargo test -p libpachinko --all-features --locked --test queue every_invariant
thread 'invariants::every_invariant_holds_after_every_step_of_any_walk' (413275351) panicked at crates/libpachinko/tests/queue/walk.rs:1110:5:
assertion `left == right` failed: the state the rules predict
  left: Model { next_position: Position(7), publishers: {PublisherId("p0"), PublisherId("p1")}, consumers: {ConsumerId("c0"): None, ConsumerId("c1"): Some(LeaseId(2))}, pockets: {PocketKey("a"): ModelPocket {
 right: Model { next_position: Position(7), publishers: {PublisherId("p0"), PublisherId("p1")}, consumers: {ConsumerId("c0"): None, ConsumerId("c1"): Some(LeaseId(2))}, pockets: {PocketKey("a"): ModelPocket {
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.01s
exit 101
=== 4 release keeps the pocket's old position
$ cargo test -p libpachinko --all-features --locked --test queue every_invariant
thread 'invariants::every_invariant_holds_after_every_step_of_any_walk' (413524477) panicked at crates/libpachinko/tests/queue/walk.rs:1110:5:
assertion `left == right` failed: the state the rules predict
  left: Model { next_position: Position(2), publishers: {PublisherId("p0")}, consumers: {ConsumerId("c0"): None, ConsumerId("c1"): None}, pockets: {PocketKey("a"): ModelPocket { status: Queued, position: Posi
 right: Model { next_position: Position(3), publishers: {PublisherId("p0")}, consumers: {ConsumerId("c0"): None, ConsumerId("c1"): None}, pockets: {PocketKey("a"): ModelPocket { status: Queued, position: Posi
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.01s
exit 101
=== 5 generator: limits capped at 1 (no batch of several)
$ cargo test -p libpachinko --all-features --locked --test queue the_walk_reaches
thread 'invariants::the_walk_reaches_every_state_and_every_outcome' (413608744) panicked at crates/libpachinko/tests/queue/invariants.rs:459:5:
assertion `left == right` failed: never reached
  left: ["batch of several"]
 right: []
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.17s
exit 101
=== restored
$ git status --short crates/libpachinko/src
$ cargo test -p libpachinko --all-features --locked --test queue every_invariant
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.15s
exit 0
$ cargo test -p libpachinko --all-features --locked --test queue the_walk_reaches
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 95 filtered out; finished in 0.19s
exit 0
```

Break 2: the prediction parks pocket `b` (`status: Parked`, out of the pocket order);
the queue left it `Queued`. Break 3: lease 1 is predicted ended and gone; the queue kept
it active. Break 4: the prediction moves the released pocket to position 2 with
`next_position` 3; the queue kept position 0 and `next_position` 2. Break 5 removes a label
from the generator's reach (limits capped at 1), and the reachability test names it.

### Step 6: the catalog and the obligations

`docs/reference/testing.md` gains the "Scenario catalog" (76 rows; the owning test by file
and function, S005 by its `compile_fail` doctest on `Config`, the ten wire tests by their
`#[tokio::test]` and the driver that carries the tag), the walk in "Tests from the
specifications", the regression-file paragraph in "Layout" and two rows in "What the
current suite proves". `configuration.md`, `maintain-dependencies.md` and decision 0007
name proptest beside divan.

`just plan-spec pachinko` prints 190 obligations, as at `611a86e`. Every one is tagged.
`rg -o 'obligation: [^ ]+' ... | sort -u | wc -l` prints 200: the 190, the eight
`guarantee.*` tags and two placeholder strings in `main.rs`'s doc comment
(the tag-convention examples, read up to the closing backtick). Handed back: nothing.
`rule-failure.RequeueDeferredRecord.2`, which the walk tags but no test can observe
(T28-2 C1), would have gone back to T25, its owner. The wave lead recorded an exception
instead, because T25 is done: the walk's tag and the header's caveat stand, and the
question is an Open point for the maintainer, which the lead carries to T31. Obligations
carried only by the
walk: the six header guards (`PullWithinLease.2` is also on `pull.rs`), the five above,
and `transition-rejected.Pocket.status` and `transition-rejected.Record.status`; the
owning tickets decide whether they also want direct tests.

The map, one id per line, each with every test that carries it:

```text
config-default.dead_letter_ttl -> crates/libpachinko/tests/queue/identity.rs::the_default_parking_threshold_and_decay_ttls_apply
config-default.lease_ttl -> crates/libpachinko/tests/queue/identity.rs::the_default_lease_ttl_and_pull_batch_apply
config-default.max_consecutive_expiries -> crates/libpachinko/tests/queue/identity.rs::the_default_parking_threshold_and_decay_ttls_apply
config-default.max_pull_batch -> crates/libpachinko/tests/queue/identity.rs::the_default_lease_ttl_and_pull_batch_apply
config-default.superseded_ttl -> crates/libpachinko/tests/queue/identity.rs::the_default_parking_threshold_and_decay_ttls_apply
entity-fields.Consumer -> crates/libpachinko/tests/queue/identity.rs::a_consumer_joins_once
entity-fields.DeadLetter -> crates/libpachinko/tests/queue/settle.rs::a_nack_dead_letters_the_held_version_on_the_first_nack_without_redelivery
entity-fields.Lease -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
entity-fields.Pocket -> crates/libpachinko/tests/queue/publish.rs::a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record
entity-fields.Publisher -> crates/libpachinko/tests/queue/identity.rs::a_publisher_joins_once
entity-fields.Queue -> crates/libpachinko/tests/queue/publish.rs::a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record
entity-fields.Record -> crates/libpachinko/tests/queue/publish.rs::a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record
entity-fields.SupersededRecord -> crates/libpachinko/tests/queue/publish.rs::a_supersede_replaces_rank_and_payload_in_place_and_sheds_the_loser
entity-relationship.Consumer.leases -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
entity-relationship.Pocket.leases -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
entity-relationship.Pocket.records -> crates/libpachinko/tests/queue/publish.rs::a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record
invariant.DeadLetter.NackedBeforeExpiry -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/settle.rs::a_nack_dead_letters_the_held_version_on_the_first_nack_without_redelivery
invariant.DeadLetterTtlNonNegative -> crates/libpachinko/src/config.rs::Config; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
invariant.ExpiryLimitPositive -> crates/libpachinko/tests/queue/identity.rs::a_zero_expiry_limit_is_refused; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
invariant.Lease.EndedAfterGrant -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/lease.rs::a_second_release_is_rejected_and_a_release_at_the_grant_instant_is_legal
invariant.Lease.GrantedBeforeExpiry -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/pull.rs::every_lease_is_granted_before_it_expires
invariant.LeaseTtlNonNegative -> crates/libpachinko/src/config.rs::Config; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
invariant.OneActiveLeasePerConsumer -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/pull.rs::each_pocket_and_each_consumer_has_at_most_one_active_lease
invariant.OneActiveLeasePerPocket -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/pull.rs::each_pocket_and_each_consumer_has_at_most_one_active_lease
invariant.OnePocketPerKey -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/publish.rs::one_pocket_per_key_holds_after_publishes_into_new_and_existing_pockets
invariant.OneRecordPerKeyPerPocket -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/publish.rs::one_record_per_key_per_pocket_holds_after_new_keys_supersedes_and_stale_publishes
invariant.OutstandingOnlyWhileLeased -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/pull.rs::held_records_exist_only_in_leased_pockets
invariant.Pocket.ExpiriesWithinLimit -> crates/libpachinko/tests/queue/expiry.rs::progress_under_a_lease_does_not_reset_the_expiry_count; crates/libpachinko/tests/queue/expiry.rs::three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it; crates/libpachinko/tests/queue/invariants.rs::?
invariant.Pocket.IdleHasNoRecords -> crates/libpachinko/tests/queue/invariants.rs::?; crates/libpachinko/tests/queue/lease.rs::a_release_idles_an_empty_pocket
invariant.Pocket.LeasedHasOneActiveLease -> crates/libpachinko/tests/queue/invariants.rs::?; crates/libpachinko/tests/queue/pull.rs::a_leased_pocket_has_exactly_one_active_lease_and_no_other_pocket_has_one
invariant.Pocket.UnleasedHasNoActiveLease -> crates/libpachinko/tests/queue/invariants.rs::?; crates/libpachinko/tests/queue/pull.rs::a_leased_pocket_has_exactly_one_active_lease_and_no_other_pocket_has_one
invariant.PocketPositionsBelowNext -> crates/libpachinko/tests/queue/expiry.rs::pockets_and_records_form_one_strict_deterministic_order; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/publish.rs::pocket_positions_are_unique_and_below_next_after_publishes
invariant.PullBatchPositive -> crates/libpachinko/tests/queue/identity.rs::a_zero_pull_batch_is_refused; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
invariant.Record.HeldVersionNotNewer -> crates/libpachinko/tests/queue/invariants.rs::?; crates/libpachinko/tests/queue/pull.rs::a_held_version_is_never_newer_than_the_record
invariant.RecordPositionsBelowNext -> crates/libpachinko/tests/queue/expiry.rs::pockets_and_records_form_one_strict_deterministic_order; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/publish.rs::record_positions_are_unique_and_below_next_after_publishes
invariant.SupersededRecord.ShedBeforeExpiry -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/publish.rs::a_supersede_replaces_rank_and_payload_in_place_and_sheds_the_loser; crates/libpachinko/tests/queue/publish.rs::every_shed_entry_is_shed_before_it_expires
invariant.SupersededTtlNonNegative -> crates/libpachinko/src/config.rs::Config; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
invariant.UniqueConsumerId -> crates/libpachinko/tests/queue/identity.rs::a_consumer_joins_once; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
invariant.UniquePocketPosition -> crates/libpachinko/tests/queue/expiry.rs::pockets_and_records_form_one_strict_deterministic_order; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/publish.rs::pocket_positions_are_unique_and_below_next_after_publishes
invariant.UniquePublisherId -> crates/libpachinko/tests/queue/identity.rs::a_publisher_joins_once; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
invariant.UniqueRecordPosition -> crates/libpachinko/tests/queue/expiry.rs::pockets_and_records_form_one_strict_deterministic_order; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/publish.rs::record_positions_are_unique_and_below_next_after_publishes
projection.Consumer.active_leases -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
projection.Pocket.active_leases -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
projection.Pocket.deferred_records -> crates/libpachinko/tests/queue/settle.rs::a_supersede_while_the_key_is_held_defers_it_on_ack
projection.Pocket.held_records -> crates/libpachinko/tests/queue/lease.rs::the_consumer_view_exposes_only_the_active_lease_and_its_held_versions; crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
projection.Pocket.queued_records -> crates/libpachinko/tests/queue/publish.rs::a_publish_of_a_new_key_appends_to_an_existing_pocket
rule-entity-creation.ConsumerJoins.1 -> crates/libpachinko/tests/queue/identity.rs::a_consumer_joins_once
rule-entity-creation.DropStalePublish.1 -> crates/libpachinko/tests/queue/publish.rs::a_stale_publish_is_shed_not_applied_and_not_an_error
rule-entity-creation.NackDefersNewerVersion.1 -> crates/libpachinko/tests/queue/settle.rs::a_nack_of_a_superseded_held_version_dead_letters_it_and_defers_the_newer
rule-entity-creation.NackSettlesRecord.1 -> crates/libpachinko/tests/queue/settle.rs::a_nack_dead_letters_the_held_version_on_the_first_nack_without_redelivery
rule-entity-creation.PublisherJoins.1 -> crates/libpachinko/tests/queue/identity.rs::a_publisher_joins_once
rule-entity-creation.PublishIntoNewPocket.1 -> crates/libpachinko/tests/queue/publish.rs::a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record
rule-entity-creation.PublishIntoNewPocket.2 -> crates/libpachinko/tests/queue/publish.rs::a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record
rule-entity-creation.PublishNewKey.1 -> crates/libpachinko/tests/queue/publish.rs::a_publish_of_a_new_key_appends_to_an_existing_pocket
rule-entity-creation.PullClaimsFrontPocket.1 -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
rule-entity-creation.Supersede.1 -> crates/libpachinko/tests/queue/publish.rs::a_supersede_replaces_rank_and_payload_in_place_and_sheds_the_loser
rule-failure.AckDefersNewerVersion.1 -> crates/libpachinko/tests/queue/settle.rs::rejected_acks_change_nothing_not_even_the_leases_expiry
rule-failure.AckDefersNewerVersion.2 -> crates/libpachinko/tests/queue/settle.rs::rejected_acks_change_nothing_not_even_the_leases_expiry
rule-failure.AckDefersNewerVersion.3 -> crates/libpachinko/tests/queue/settle.rs::rejected_acks_change_nothing_not_even_the_leases_expiry
rule-failure.AckDefersNewerVersion.4 -> crates/libpachinko/tests/queue/settle.rs::rejected_acks_change_nothing_not_even_the_leases_expiry
rule-failure.AckDefersNewerVersion.5 -> crates/libpachinko/tests/queue/settle.rs::an_ack_settles_a_held_record
rule-failure.AckSettlesRecord.1 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_that_re_claims_the_same_pocket_cannot_use_its_old_lease; crates/libpachinko/tests/queue/settle.rs::rejected_acks_change_nothing_not_even_the_leases_expiry
rule-failure.AckSettlesRecord.2 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_cannot_act_on_another_consumers_lease; crates/libpachinko/tests/queue/settle.rs::rejected_acks_change_nothing_not_even_the_leases_expiry
rule-failure.AckSettlesRecord.3 -> crates/libpachinko/tests/queue/settle.rs::rejected_acks_change_nothing_not_even_the_leases_expiry
rule-failure.AckSettlesRecord.4 -> crates/libpachinko/tests/queue/settle.rs::rejected_acks_change_nothing_not_even_the_leases_expiry
rule-failure.AckSettlesRecord.5 -> crates/libpachinko/tests/queue/settle.rs::a_supersede_while_the_key_is_held_defers_it_on_ack
rule-failure.ConsumerJoins.1 -> crates/libpachinko/tests/queue/identity.rs::a_consumer_joins_once
rule-failure.DropStalePublish.1 -> crates/libpachinko/tests/queue/publish.rs::one_pocket_per_key_holds_after_publishes_into_new_and_existing_pockets
rule-failure.DropStalePublish.2 -> crates/libpachinko/tests/queue/publish.rs::one_record_per_key_per_pocket_holds_after_new_keys_supersedes_and_stale_publishes
rule-failure.DropStalePublish.3 -> crates/libpachinko/tests/queue/expiry.rs::an_equal_rank_publish_does_not_revive_a_parked_pocket; crates/libpachinko/tests/queue/publish.rs::an_equal_rank_publish_is_rejected_as_the_open_question_1_placeholder; crates/libpachinko/tests/queue/publish.rs::negative_and_zero_ranks_order_like_integers
rule-failure.HoldRecord.1 -> crates/libpachinko/tests/queue/pull.rs::a_held_record_is_not_handed_out_again_under_the_same_lease
rule-failure.HoldRecord.2 -> crates/libpachinko/tests/queue/pull.rs::a_held_record_is_not_handed_out_again_under_the_same_lease
rule-failure.LeaseExpiryIdlesEmptyPocket.1 -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_fires_once_per_lease
rule-failure.LeaseExpiryIdlesEmptyPocket.2 -> crates/libpachinko/tests/queue/expiry.rs::a_lease_lapses_exactly_at_expires_at
rule-failure.LeaseExpiryIdlesEmptyPocket.3 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.LeaseExpiryIdlesEmptyPocket.4 -> crates/libpachinko/tests/queue/expiry.rs::at_the_threshold_an_empty_pocket_parks_rather_than_idling
rule-failure.LeaseExpiryIdlesEmptyPocket.5 -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_keeps_the_pocket_at_the_front_and_requeues_its_records
rule-failure.LeaseExpiryKeepsPocketAtFront.1 -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_fires_once_per_lease
rule-failure.LeaseExpiryKeepsPocketAtFront.2 -> crates/libpachinko/tests/queue/expiry.rs::a_lease_lapses_exactly_at_expires_at
rule-failure.LeaseExpiryKeepsPocketAtFront.3 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.LeaseExpiryKeepsPocketAtFront.4 -> crates/libpachinko/tests/queue/expiry.rs::three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it
rule-failure.LeaseExpiryKeepsPocketAtFront.5 -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_with_no_records_idles_the_pocket
rule-failure.LeaseExpiryParksPocket.1 -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_fires_once_per_lease
rule-failure.LeaseExpiryParksPocket.2 -> crates/libpachinko/tests/queue/expiry.rs::a_lease_lapses_exactly_at_expires_at
rule-failure.LeaseExpiryParksPocket.3 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.LeaseExpiryParksPocket.4 -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_keeps_the_pocket_at_the_front_and_requeues_its_records
rule-failure.LeaseLapses.1 -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_fires_once_per_lease
rule-failure.NackDefersNewerVersion.1 -> crates/libpachinko/tests/queue/settle.rs::rejected_nacks_change_nothing_and_create_no_dead_letter
rule-failure.NackDefersNewerVersion.2 -> crates/libpachinko/tests/queue/settle.rs::rejected_nacks_change_nothing_and_create_no_dead_letter
rule-failure.NackDefersNewerVersion.3 -> crates/libpachinko/tests/queue/settle.rs::rejected_nacks_change_nothing_and_create_no_dead_letter
rule-failure.NackDefersNewerVersion.4 -> crates/libpachinko/tests/queue/settle.rs::rejected_nacks_change_nothing_and_create_no_dead_letter
rule-failure.NackDefersNewerVersion.5 -> crates/libpachinko/tests/queue/settle.rs::a_nack_dead_letters_the_held_version_on_the_first_nack_without_redelivery
rule-failure.NackSettlesRecord.1 -> crates/libpachinko/tests/queue/settle.rs::rejected_nacks_change_nothing_and_create_no_dead_letter
rule-failure.NackSettlesRecord.2 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_cannot_act_on_another_consumers_lease; crates/libpachinko/tests/queue/settle.rs::rejected_nacks_change_nothing_and_create_no_dead_letter
rule-failure.NackSettlesRecord.3 -> crates/libpachinko/tests/queue/settle.rs::rejected_nacks_change_nothing_and_create_no_dead_letter
rule-failure.NackSettlesRecord.4 -> crates/libpachinko/tests/queue/settle.rs::rejected_nacks_change_nothing_and_create_no_dead_letter
rule-failure.NackSettlesRecord.5 -> crates/libpachinko/tests/queue/settle.rs::a_nack_of_a_superseded_held_version_dead_letters_it_and_defers_the_newer
rule-failure.PublisherJoins.1 -> crates/libpachinko/tests/queue/identity.rs::a_publisher_joins_once
rule-failure.PublishIntoNewPocket.1 -> crates/libpachinko/tests/queue/publish.rs::a_publish_of_a_new_key_appends_to_an_existing_pocket; crates/libpachinko/tests/queue/publish.rs::the_same_record_key_in_two_pockets_is_two_records
rule-failure.PublishNewKey.1 -> crates/libpachinko/tests/queue/publish.rs::the_same_record_key_in_two_pockets_is_two_records
rule-failure.PublishNewKey.2 -> crates/libpachinko/tests/queue/publish.rs::a_supersede_replaces_rank_and_payload_in_place_and_sheds_the_loser
rule-failure.PullClaimsFrontPocket.1 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_with_a_lease_never_gets_no_pocket_available_or_another_pocket; crates/libpachinko/tests/queue/pull.rs::a_pull_under_a_lease_extends_the_lease
rule-failure.PullClaimsFrontPocket.2 -> crates/libpachinko/tests/queue/expiry.rs::idle_parked_and_leased_pockets_are_never_claimed; crates/libpachinko/tests/queue/pull.rs::a_pull_with_no_queued_pocket_returns_no_pocket_available_not_an_error
rule-failure.PullClaimsFrontPocket.3 -> crates/libpachinko/tests/queue/pull.rs::a_leased_pocket_has_exactly_one_active_lease_and_no_other_pocket_has_one
rule-failure.PullFindsNoPocket.1 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_with_a_lease_never_gets_no_pocket_available_or_another_pocket; crates/libpachinko/tests/queue/pull.rs::a_pull_with_no_queued_pocket_returns_no_pocket_available_not_an_error
rule-failure.PullFindsNoPocket.2 -> crates/libpachinko/tests/queue/pull.rs::a_second_consumer_never_receives_records_from_a_leased_pocket
rule-failure.PullWithinLease.1 -> crates/libpachinko/tests/queue/pull.rs::a_pull_under_a_lease_extends_the_lease
rule-failure.PullWithinLease.2 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/pull.rs::a_leased_pocket_has_exactly_one_active_lease_and_no_other_pocket_has_one
rule-failure.ReleaseIdlesEmptyPocket.1 -> crates/libpachinko/tests/queue/lease.rs::a_second_release_is_rejected_and_a_release_at_the_grant_instant_is_legal
rule-failure.ReleaseIdlesEmptyPocket.2 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_cannot_act_on_another_consumers_lease
rule-failure.ReleaseIdlesEmptyPocket.3 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.ReleaseIdlesEmptyPocket.4 -> crates/libpachinko/tests/queue/lease.rs::a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place
rule-failure.ReleaseRequeuesPocket.1 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_that_re_claims_the_same_pocket_cannot_use_its_old_lease; crates/libpachinko/tests/queue/lease.rs::a_second_release_is_rejected_and_a_release_at_the_grant_instant_is_legal
rule-failure.ReleaseRequeuesPocket.2 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_cannot_act_on_another_consumers_lease
rule-failure.ReleaseRequeuesPocket.3 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.ReleaseRequeuesPocket.4 -> crates/libpachinko/tests/queue/lease.rs::a_release_idles_an_empty_pocket
rule-failure.Renew.1 -> crates/libpachinko/tests/queue/lease.rs::a_consumer_that_re_claims_the_same_pocket_cannot_use_its_old_lease; crates/libpachinko/tests/queue/lease.rs::a_renew_of_an_ended_lease_is_rejected_and_moves_nothing
rule-failure.Renew.2 -> crates/libpachinko/tests/queue/expiry.rs::after_a_lapse_the_old_lease_is_fenced_even_when_the_key_has_a_newer_rank; crates/libpachinko/tests/queue/lease.rs::a_consumer_cannot_act_on_another_consumers_lease
rule-failure.RequeueDeferredRecord.1 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.RequeueDeferredRecord.2 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.RequeueHeldRecord.1 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.RequeueHeldRecord.2 -> crates/libpachinko/tests/queue/lease.rs::a_release_with_only_deferred_records_requeues_the_pocket_rather_than_idling_it
rule-failure.ReviveIdlePocket.1 -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
rule-failure.ReviveIdlePocket.2 -> crates/libpachinko/tests/queue/lease.rs::a_publish_revives_an_idle_pocket_at_the_back
rule-failure.ReviveParkedPocket.1 -> crates/libpachinko/tests/queue/expiry.rs::a_supersede_or_a_stale_publish_revives_a_parked_pocket
rule-failure.ReviveParkedPocket.2 -> crates/libpachinko/tests/queue/expiry.rs::only_a_publish_revives_a_parked_pocket
rule-failure.Supersede.1 -> crates/libpachinko/tests/queue/publish.rs::one_pocket_per_key_holds_after_publishes_into_new_and_existing_pockets
rule-failure.Supersede.2 -> crates/libpachinko/tests/queue/publish.rs::one_record_per_key_per_pocket_holds_after_new_keys_supersedes_and_stale_publishes
rule-failure.Supersede.3 -> crates/libpachinko/tests/queue/expiry.rs::an_equal_rank_publish_does_not_revive_a_parked_pocket; crates/libpachinko/tests/queue/publish.rs::a_stale_publish_is_shed_not_applied_and_not_an_error
rule-success.AckDefersNewerVersion -> crates/libpachinko/tests/queue/settle.rs::a_supersede_while_the_key_is_held_defers_it_on_ack
rule-success.AckSettlesRecord -> crates/libpachinko/tests/queue/expiry.rs::an_ack_after_expires_at_but_before_the_lapse_succeeds; crates/libpachinko/tests/queue/settle.rs::an_ack_extends_the_lease; crates/libpachinko/tests/queue/settle.rs::an_ack_settles_a_held_record
rule-success.ConsumerJoins -> crates/libpachinko/tests/queue/identity.rs::a_consumer_joins_once
rule-success.DeadLetterDecays -> crates/libpachinko/tests/queue/decay.rs::dead_letters_decay_at_nacked_at_plus_dead_letter_ttl; crates/libpachinko/tests/queue/decay.rs::zero_superseded_and_dead_letter_ttls_decay_at_once
rule-success.DropStalePublish -> crates/libpachinko/tests/queue/publish.rs::a_stale_publish_is_shed_not_applied_and_not_an_error; crates/libpachinko/tests/queue/settle.rs::a_stale_check_while_held_compares_with_the_latest_rank_not_the_held_one
rule-success.HoldRecord -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
rule-success.LeaseExpiryIdlesEmptyPocket -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_with_no_records_idles_the_pocket
rule-success.LeaseExpiryKeepsPocketAtFront -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_keeps_the_pocket_at_the_front_and_requeues_its_records
rule-success.LeaseExpiryParksPocket -> crates/libpachinko/tests/queue/expiry.rs::three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it
rule-success.LeaseLapses -> crates/libpachinko/tests/queue/expiry.rs::a_zero_lease_ttl_lapses_at_the_instant_of_grant
rule-success.NackDefersNewerVersion -> crates/libpachinko/tests/queue/settle.rs::a_nack_of_a_superseded_held_version_dead_letters_it_and_defers_the_newer
rule-success.NackSettlesRecord -> crates/libpachinko/tests/queue/settle.rs::a_nack_dead_letters_the_held_version_on_the_first_nack_without_redelivery
rule-success.PublisherJoins -> crates/libpachinko/tests/queue/identity.rs::a_publisher_joins_once
rule-success.PublishIntoNewPocket -> crates/libpachinko/tests/queue/publish.rs::a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record
rule-success.PublishNewKey -> crates/libpachinko/tests/queue/publish.rs::a_publish_of_a_new_key_appends_to_an_existing_pocket
rule-success.PullClaimsFrontPocket -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
rule-success.PullFindsNoPocket -> crates/libpachinko/tests/queue/pull.rs::a_pull_with_no_queued_pocket_returns_no_pocket_available_not_an_error
rule-success.PullWithinLease -> crates/libpachinko/tests/queue/expiry.rs::a_pull_after_expires_at_but_before_the_lapse_extends_the_lease; crates/libpachinko/tests/queue/lease.rs::a_consumer_with_a_lease_never_gets_no_pocket_available_or_another_pocket; crates/libpachinko/tests/queue/pull.rs::a_pull_under_a_lease_extends_the_lease
rule-success.ReleaseIdlesEmptyPocket -> crates/libpachinko/tests/queue/lease.rs::a_release_idles_an_empty_pocket
rule-success.ReleaseRequeuesPocket -> crates/libpachinko/tests/queue/expiry.rs::a_release_resets_the_consecutive_expiry_count; crates/libpachinko/tests/queue/lease.rs::a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place; crates/libpachinko/tests/queue/lease.rs::a_release_with_only_deferred_records_requeues_the_pocket_rather_than_idling_it
rule-success.Renew -> crates/libpachinko/tests/queue/lease.rs::a_renew_extends_the_lease_to_now_plus_lease_ttl; crates/libpachinko/tests/queue/lease.rs::renewals_do_not_stack
rule-success.RequeueDeferredRecord -> crates/libpachinko/tests/queue/expiry.rs::a_supersede_while_held_is_deferred_on_nack_and_the_lapse_frees_it; crates/libpachinko/tests/queue/lease.rs::a_release_with_only_deferred_records_requeues_the_pocket_rather_than_idling_it; crates/libpachinko/tests/queue/settle.rs::a_deferred_record_restarts_at_the_back_of_its_pocket
rule-success.RequeueHeldRecord -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_keeps_the_pocket_at_the_front_and_requeues_its_records; crates/libpachinko/tests/queue/lease.rs::a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place
rule-success.ReviveIdlePocket -> crates/libpachinko/tests/queue/lease.rs::a_publish_revives_an_idle_pocket_at_the_back
rule-success.ReviveParkedPocket -> crates/libpachinko/tests/queue/expiry.rs::three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it
rule-success.Supersede -> crates/libpachinko/tests/queue/publish.rs::a_supersede_replaces_rank_and_payload_in_place_and_sheds_the_loser; crates/libpachinko/tests/queue/settle.rs::two_supersedes_while_held_shed_both_losers_and_leave_the_held_version
rule-success.SupersededRecordDecays -> crates/libpachinko/tests/queue/decay.rs::superseded_entries_decay_at_shed_at_plus_superseded_ttl; crates/libpachinko/tests/queue/decay.rs::zero_superseded_and_dead_letter_ttls_decay_at_once
surface-actor.Consuming -> crates/libpachinko/tests/queue/identity.rs::a_pull_or_lease_read_by_a_consumer_that_never_joined_is_refused_and_changes_nothing
surface-actor.Publishing -> crates/libpachinko/tests/queue/identity.rs::a_publish_by_a_publisher_that_never_joined_is_refused_and_changes_nothing
surface-exposure.Consuming -> crates/libpachinko/tests/queue/lease.rs::the_consumer_view_exposes_only_the_active_lease_and_its_held_versions; crates/libpachinko/tests/queue/pull.rs::a_pull_under_a_lease_extends_the_lease
surface-provides.Consuming -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
surface-provides.Publishing -> crates/libpachinko/tests/queue/publish.rs::a_publish_into_a_new_pocket_creates_a_queued_pocket_and_record
temporal.DeadLetterDecays -> crates/libpachinko/tests/queue/decay.rs::dead_letters_decay_at_nacked_at_plus_dead_letter_ttl
temporal.LeaseLapses -> crates/libpachinko/tests/queue/expiry.rs::a_lease_lapses_exactly_at_expires_at
temporal.SupersededRecordDecays -> crates/libpachinko/tests/queue/decay.rs::superseded_entries_decay_at_shed_at_plus_superseded_ttl
transition-edge.Lease.active.expired -> crates/libpachinko/tests/queue/expiry.rs::a_lease_lapses_exactly_at_expires_at
transition-edge.Lease.active.released -> crates/libpachinko/tests/queue/expiry.rs::a_release_after_expires_at_but_before_the_lapse_succeeds; crates/libpachinko/tests/queue/lease.rs::a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place
transition-edge.Pocket.idle.queued -> crates/libpachinko/tests/queue/lease.rs::a_publish_revives_an_idle_pocket_at_the_back
transition-edge.Pocket.leased.idle -> crates/libpachinko/tests/queue/lease.rs::a_release_idles_an_empty_pocket
transition-edge.Pocket.leased.parked -> crates/libpachinko/tests/queue/expiry.rs::three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it
transition-edge.Pocket.leased.queued -> crates/libpachinko/tests/queue/lease.rs::a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place
transition-edge.Pocket.parked.queued -> crates/libpachinko/tests/queue/expiry.rs::three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it
transition-edge.Pocket.queued.leased -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
transition-edge.Record.deferred.queued -> crates/libpachinko/tests/queue/settle.rs::a_deferred_record_restarts_at_the_back_of_its_pocket
transition-edge.Record.held.deferred -> crates/libpachinko/tests/queue/settle.rs::a_supersede_while_the_key_is_held_defers_it_on_ack
transition-edge.Record.held.queued -> crates/libpachinko/tests/queue/lease.rs::a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place
transition-edge.Record.queued.held -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
transition-rejected.Lease.status -> crates/libpachinko/tests/queue/expiry.rs::a_released_lease_never_lapses_afterwards; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
transition-rejected.Pocket.status -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
transition-rejected.Record.status -> crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk
transition-terminal.Lease.status -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_fires_once_per_lease; crates/libpachinko/tests/queue/invariants.rs::every_invariant_holds_after_every_step_of_any_walk; crates/libpachinko/tests/queue/lease.rs::a_renew_of_an_ended_lease_is_rejected_and_moves_nothing; crates/libpachinko/tests/queue/lease.rs::a_second_release_is_rejected_and_a_release_at_the_grant_instant_is_legal
when-presence.Lease.ended_at -> crates/libpachinko/tests/queue/lease.rs::a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place
when-presence.Record.held_payload -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
when-presence.Record.held_rank -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
when-set.HoldRecord.Record.held_payload -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
when-set.HoldRecord.Record.held_rank -> crates/libpachinko/tests/queue/pull.rs::a_pull_claims_the_front_pocket_leases_it_and_holds_the_batch
when-set.LeaseExpiryIdlesEmptyPocket.Lease.ended_at -> crates/libpachinko/tests/queue/expiry.rs::a_lapse_with_no_records_idles_the_pocket
when-set.LeaseExpiryKeepsPocketAtFront.Lease.ended_at -> crates/libpachinko/tests/queue/expiry.rs::a_lease_lapses_exactly_at_expires_at
when-set.LeaseExpiryParksPocket.Lease.ended_at -> crates/libpachinko/tests/queue/expiry.rs::three_consecutive_lapses_park_the_pocket_and_a_publish_revives_it
when-set.ReleaseIdlesEmptyPocket.Lease.ended_at -> crates/libpachinko/tests/queue/lease.rs::a_release_idles_an_empty_pocket
when-set.ReleaseRequeuesPocket.Lease.ended_at -> crates/libpachinko/tests/queue/lease.rs::a_release_requeues_the_pocket_at_the_back_and_its_held_records_in_place
```

### T28-2: the whole diff (step 7)

Harness and model: two reviewer lenses, `conformance` and `breakage`, each
`codex exec -m gpt-6-astra -s read-only` (exit 0), with separate triage and one writer,
run by the wave lead; outputs under `ai_tmp/review/T28-2/`. 3 findings, 2 valid, none
high, so no second round.

| Id | Verdict | Reason |
| --- | --- | --- |
| B1 | invalid, low | The walk's `RequeueDeferredRecord.2` tag: no observable state goes unverified, and the header and hand-back already state the limit |
| C1 | valid, low (from medium) | The hand-back said the walk's prediction shows `RequeueDeferredRecord.2` and handed nothing back, though step 6 sends an unobservable id to its owner |
| C2 | valid, low | The break quotations lacked each command and its exit status (§11) |

Applied by the writer: the hand-back separates `RequeueDeferredRecord.2` from the four the
prediction shows (C1); the five breaks were rerun and quoted with each command, its
`exit 101` and the restored tree's `exit 0` (C2). Handed back: nothing. On C1 the wave
lead recorded an exception: `RequeueDeferredRecord.2` is not handed back to T25, which is
done, and is an Open point for the maintainer with its mirror `RequeueHeldRecord.2`,
carried to T31.

Rerun after the fixes, at `1fdaab3` plus the uncommitted change, `just check` (exit 0):

```text
All checks passed and the worktree is unchanged.
```

## Open points

- `rule-failure.RequeueDeferredRecord.2` and its mirror `rule-failure.RequeueHeldRecord.2`
  are obligations whose rejection no test can observe. `Freed` is emitted for every held
  and every deferred record, so the rule for the other status is always reached and
  rejected, and a wrongly accepted rule would leave the record queued, which is the state
  the matching rule produces anyway. A specification-level question for the maintainer:
  record them as equivalent obligations, or narrow each rule's trigger so the guard is
  not needed. The wave lead recorded this exception in place of a hand-back to T25,
  which is done, and carries it to T31. The walk tags `.2` and T25's S054 test tags the
  mirror; both tags stay.
- `docs/reference/testing.md` says today that a property test's failing case is written
  to a `proptest-regressions/` file and committed. The walk persists nothing, so that a
  red run never writes into the worktree a read-only recipe is snapshotting, and
  reproduces a failure from the seed and the shrunk case instead; this ticket rewrites the
  paragraph. If the maintainer prefers committed regression files, the runner takes
  `failure_persistence` back and the paragraph stands.
- The walk's domains are small on purpose (three pockets, three keys, ranks 0 to 3) so
  that collisions, supersedes and revives are frequent. A wider walk at lower weights is
  a later choice, not a defect; the reachability list is what decides whether the
  generator is blind.
- An obligation whose only test is this walk is listed in the hand-back; the owning
  ticket decides whether it also wants a direct test.
- The walk runs over `MemoryRecords`. Running it over `LogRecords` (T21, 0019's reopen
  option) is a one-line instantiation when that store lands; nothing here is done for
  it.
- The entity read-back tests of PR 39 (`every_pocket_and_record_field_reads_back` and
  the lease and record projection tests) are added here only if the obligation
  reconciliation finds their ids untagged.
