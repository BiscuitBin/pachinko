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

Not started. The executing agent writes this section: each quoted output headed by the
commit it ran at, with its exit status (`CONVENTIONS.md` §11), and for each adversarial
checkpoint the harness and model, the findings, each verdict, what was applied and what
was handed back (`CONVENTIONS.md` §14).

## Open points

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
