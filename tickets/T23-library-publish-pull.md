---
id: T23
title: "Library tracer: effects, the store seam, publish and pull"
status: todo
depends_on: [T22]
parallel_with: []
branch: ticket/t23-library-publish-pull
estimated_size: L
---

# T23: Library tracer: effects, the store seam, publish and pull

## Context

Decision 0019 (T22) settled the runtime, the executor, the store seam, the lease-id
scheme and the clock; the decision table in T22's Context is the authority this ticket
builds to, and `docs/specs/pachinko.allium` is the authority on behaviour. This ticket
is the library tracer: it lands the two effect traits with their fakes, the library-owned
store trait with its one contract and its in-memory implementation, the `Queue` with its
identity, publish and pull rules, and the test layout every later library ticket extends.
T24 builds the service on these shapes, T25 adds the lease actions, T26 the lapse and the
tick, T28 the invariant walk. The shapes are therefore reviewed in the middle of the
ticket, before the rules are filled in, because they are what the wave builds on.

Ownership (the decision table's rule, restated): a rule, invariant, guarantee or scenario
is owned by the ticket whose test pins it and whose hand-back names it. Code for a rule
may land earlier than its owner: this ticket lands the `PocketTouched` dispatch with its
idle and parked arms, and T25 and T26 own `ReviveIdlePocket` and `ReviveParkedPocket`,
because an idle pocket and a parked pocket first exist there.

The prototypes this ticket reads are heads of unmerged branches; a fresh worktree needs
`git fetch origin` (network, authorised by this step) before `git show` can read them.
Never check a prototype branch out into the ticket's worktree; read with
`git show <head>:<path>`.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 38, the Flight service prototype | `tickets/t20-pachinko-prototype-complete` | `cb77f07` | `git show cb77f07:<path>` |
| PR 39, the HTTP service prototype | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` | `git show cf1a6b2:<path>` |

The prototypes are sources, not the design: each line cited below is where a shape was
tried, and the design in Steps is this ticket's. Where the two differ from it, the
difference is deliberate: PR 38's `check_contract` takes a batch-reading function and
this ticket's takes none; PR 39's errors are one enum and this ticket's are split by
operation; PR 39's `Clock` is `Send + Sync` and this ticket's is also `'static`, `Clone`
and `Debug` (invariant 3).

## Goal

`libpachinko` exposes `Timestamp`, `Clock` and `FakeClock`; `LeaseId`, `LeaseIds` and
`CountingLeaseIds`; the id newtypes; `Config` with the specification's defaults and its
load checks; the error enums; `PocketRecords`, `MemoryRecords` and `check_contract`; and
`Queue` with `publisher_joins`, `consumer_joins`, `publish`, `pull` and `lease_view`.
Every public item is `Send + Sync + 'static`, `Clone`, `Debug` and `#[non_exhaustive]`
where it is a struct or enum, and carries a doctest. The tests under
`crates/libpachinko/tests/queue/` pin what this ticket owns, tagged by obligation and
scenario id; `just effects` proves the clock is the first effect behind a trait; `just
coverage` is at or above 90%; `just check` is green.

### Owns

This ticket owns 10 rules, 21 invariants, 4 guarantees and 31 scenarios. A reviewer
greps these tables against the hand-back notes and the `// obligation:` and
`// scenario:` tags.

Rules (10): `PublisherJoins`, `ConsumerJoins`, `PublishIntoNewPocket`, `PublishNewKey`,
`Supersede`, `DropStalePublish`, `PullWithinLease`, `PullClaimsFrontPocket`, `HoldRecord`,
`PullFindsNoPocket`.

Invariants (21). Five are config load checks: `ExpiryLimitPositive`, `PullBatchPositive`,
`LeaseTtlNonNegative`, `SupersededTtlNonNegative`, `DeadLetterTtlNonNegative`. Sixteen
are state invariants: `UniquePublisherId`, `UniqueConsumerId`, `OnePocketPerKey`,
`OneRecordPerKeyPerPocket`, `UniquePocketPosition`, `UniqueRecordPosition`,
`PocketPositionsBelowNext`, `RecordPositionsBelowNext`, `LeasedHasOneActiveLease`,
`UnleasedHasNoActiveLease`, `OneActiveLeasePerPocket`, `OneActiveLeasePerConsumer`,
`GrantedBeforeExpiry`, `HeldVersionNotNewer`, `OutstandingOnlyWhileLeased`,
`ShedBeforeExpiry`. T25 owns `IdleHasNoRecords`, `EndedAfterGrant` and
`NackedBeforeExpiry`; T26 owns `ExpiriesWithinLimit`; T28's walk re-checks all 25.

Guarantees (4): `EveryDropIsVisible`, `OldestSlotNewestPayload`, `ExclusivePocket`,
`OldestFirst`.

Scenarios (31). The five tagged wire are additionally exercised over HTTP by T24; that
is T24's exercise, not its ownership.

| Id | Description | Tags |
| --- | --- | --- |
| S001 | Default `lease_ttl` and `max_pull_batch` apply | core |
| S002 | Default parking threshold and decay TTLs apply | core |
| S003 | `max_consecutive_expiries = 0` is refused | edge |
| S004 | `max_pull_batch = 0` is refused | edge |
| S005 | Negative TTLs are refused | edge |
| S008 | `max_pull_batch = 1` caps every pull at one record | edge |
| S009 | A publisher joins once | core |
| S010 | A consumer joins once | core |
| S011 | Publish into a new pocket creates a queued pocket and record | core |
| S012 | Publish of a new key appends to an existing pocket | core |
| S013 | The same record key in two pockets is two records | edge |
| S014 | Supersede replaces rank and payload in place | core |
| S015 | Repeated supersede keeps the first slot and sheds every loser | edge |
| S016 | A stale publish is shed, not applied and not an error | core |
| S017 | An equal-rank publish is rejected (OQ1 placeholder, labelled) | oq:1 |
| S018 | Negative and zero ranks order like integers | edge |
| S019 | Large ranks keep full precision across supersede and stale | wire |
| S020 | Payload bytes and keys round-trip unchanged | wire |
| S021 | Publishing into a leased pocket neither requeues it nor escapes the lease | core |
| S022 | Publishing into a queued pocket does not move it in the pocket order | edge |
| S027 | A pull claims the front pocket, leases it and holds the batch | core |
| S028 | A positive limit returns at most that many, oldest first, and later pulls continue | core, wire |
| S029 | A limit above `max_pull_batch` is capped | edge |
| S030 | A limit above the queued count returns every queued record | edge |
| S031 | A zero-limit pull claims a pocket and returns an empty batch | edge, wire |
| S032 | A negative limit behaves like zero | edge |
| S033 | A pull with no queued pocket returns `NoPocketAvailable`, not an error | core, wire |
| S036 | A pull under a lease extends the lease | core |
| S037 | A held record is not handed out again under the same lease | core |
| S038 | Pockets are claimed in the order they entered | core |
| S039 | A second consumer never receives records from a leased pocket | core |

## Non-goals

- No ack, nack, renew or release (T25), no lapse, tick or decay (T26), no walk (T28):
  the shed queue and the lease registry land with their types and the `PocketTouched`
  dispatch lands with its arms, but nothing in this ticket reaches an idle or parked
  pocket, an ended lease or a decayed entry, and nothing is tagged as such.
- No service code: `crates/pachinko/` is untouched, `SystemTime` and the RNG arrive with
  T24. The `thiserror` dependency is added to the workspace table and to `libpachinko`
  only; `pachinko` takes `{ workspace = true }` in T24 when it first derives an error,
  since `just deps-unused` would fail on an unused dependency here.
- No serde, no wire types, no base64: the library stays serde-free (the decision table).
- No answer to open question 1: an equal-rank publish is `PublishError::EqualRank`,
  labelled a placeholder in its docs and its test.
- No `LogRecords`, no `ArrowRecords`, no second store: `MemoryRecords` is the one
  implementation and `check_contract` is written to admit another.
- No edit to the `Justfile`, `pyproject.toml` or `clippy.toml`: every recipe this ticket
  needs exists.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | shared, own lines | `thiserror = "2.0.21"` in `[workspace.dependencies]` with its decision-0007 note |
| `Cargo.lock` | shared, own lines | Regenerated by `just lock` |
| `crates/libpachinko/Cargo.toml` | shared, own lines | `thiserror = { workspace = true }`; `[features] testing = []` with its comment |
| `crates/libpachinko/src/lib.rs` | replaced | Crate docs describing the domain and the modules; `version()` stays; `pub mod` lines |
| `crates/libpachinko/src/clock.rs` | new | `Timestamp`, `Clock`, `FakeClock` |
| `crates/libpachinko/src/lease_ids.rs` | new | `LeaseId`, `ParseLeaseIdError`, `LeaseIds`, `CountingLeaseIds` |
| `crates/libpachinko/src/ids.rs` | new | `PublisherId`, `ConsumerId`, `PocketKey`, `RecordKey`, `Rank`, `Payload`, `Position` |
| `crates/libpachinko/src/config.rs` | new | `Config`, its defaults, `Config::check`, `ConfigError` is in `error.rs` |
| `crates/libpachinko/src/error.rs` | new | `JoinError`, `PublishError`, `PullError`, `ConfigError` |
| `crates/libpachinko/src/records.rs` | new | `PocketRecords`, `RecordStatus`, `HeldRecord`, `RecordView`; `pub mod memory`, `pub mod contract` |
| `crates/libpachinko/src/records/memory.rs` | new | `MemoryRecords` |
| `crates/libpachinko/src/records/contract.rs` | new | `check_contract`, behind `testing` |
| `crates/libpachinko/src/queue.rs` | new | `Queue`, `Publication`, `PublishOutcome`, `PullOutcome`, `LeaseView`, `Snapshot`; the submodules |
| `crates/libpachinko/src/queue/` | new | Submodules as the implementation divides them within the metrics gate: pockets and the pocket order, the lease registry, the shed queue, publish, pull, the view |
| `crates/libpachinko/tests/queue/main.rs` | new | The `#[cfg(test)]` module list and the tag conventions |
| `crates/libpachinko/tests/queue/fixture.rs` | new | The queue over `MemoryRecords`, `FakeClock` and `CountingLeaseIds`; helpers |
| `crates/libpachinko/tests/queue/identity.rs` | new | `PublisherJoins`, `ConsumerJoins`, the config scenarios |
| `crates/libpachinko/tests/queue/publish.rs` | new | The four publish rules and their scenarios |
| `crates/libpachinko/tests/queue/pull.rs` | new | The four pull rules and their scenarios |
| `crates/libpachinko/tests/records_contract.rs` | new | `check_contract::<MemoryRecords>()` |
| `AGENTS.md` | shared, own lines | Invariant 10 in the Invariants list |
| `docs/reference/agent-contract.md` | shared, own lines | Line 40: ten invariants |
| `.agents/skills/rust-change/SKILL.md` | shared, own lines | One line: errors are `thiserror` enums |
| `.agents/skills/review-change/SKILL.md` | shared, own lines | Step 2: ten invariants, the tenth named |
| `docs/explanation/architecture.md` | shared, own lines | A "The library" section: the modules, the effects traits and fakes, the store seam, the ownership rule for `PocketTouched` |
| `docs/reference/testing.md` | shared, own lines | "Tests from the specifications": the `testing` feature, the tag conventions, the queue test layout; the placeholder sentence removed |
| `docs/reference/api.md` | shared, own lines | "What the API promises", where it names items |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Added |
| `crates/libpachinko/README.md` | replaced | The domain, the effects, the store seam, the `testing` feature |
| `tickets/T23-library-publish-pull.md` | ticket | `status:` and hand-back notes |

The bridges under `.claude/skills/` and `.codex/skills/` are untouched: they repeat the
frozen frontmatter and nothing else. `crates/libpachinko/benches/version.rs` and
`fuzz/` are untouched.

## Steps

1. Create the worktree from `main` after T22 has merged (`README.md`, "How to pick up a
   ticket"), or, in a clean worktree already initialized on `main` whose branch carries
   nothing, create the branch there and skip `just initialize`. Run `just check` on the
   untouched worktree and quote its last line as the baseline. Run `git fetch origin`
   (network, authorised by this step) if `git show cb77f07:crates/libpachinko/src/records.rs`
   cannot read the PR 38 head.
2. **The plan.** Use the `plan-change` skill. Read `docs/specs/pachinko.allium` whole,
   T22's decision table, and the prototype files cited in steps 4 to 6 with `git show`.
   Run `just plan-spec pachinko` and keep its output: every test this ticket writes names
   the obligation ids it covers from that list, and the hand-back lists each id this
   ticket's tests carry. Decide the `queue/` submodule split against `rustqual.toml`'s
   limits before writing, so no file is split under the metrics gate later.
3. **The dependency.** Use the `rust-change` skill for every step from here to 8. Add
   `thiserror = "2.0.21"` to `[workspace.dependencies]` in the root `Cargo.toml`, re-pinned
   to the current release as a full `x.y.z`, under the decision-0007 note in the form the
   root `Cargo.toml` uses for divan (PR 39 root `Cargo.toml` carries the wording: it
   derives libpachinko's and pachinko's error enums, invariant 10, and a hand-written
   `Display` and `Error` per variant is what it saves). `crates/libpachinko/Cargo.toml`
   takes `thiserror = { workspace = true }` and the feature block from PR 38
   `crates/libpachinko/Cargo.toml` line 38, `[features] testing = []`, with its comment.
   Run `just lock` (network, authorised by this step), then `just lock-check` and
   `just deny`.
4. **The effects and the ids.** Five modules, each with a doctest on every public item:
   - `clock.rs`: `Timestamp`, milliseconds since the Unix epoch as a `u64`, with
     `from_millis`, `as_millis` and `saturating_add_millis(u64)`, and `Add<Duration>`
     through it; the `Clock` trait, `fn now(&self) -> Timestamp`, bounded
     `Clone + Debug + Send + Sync + 'static`; `FakeClock`, shared through an `Arc` so a
     clone and its original read one instant, with `new`, `set` and `advance`. The
     prototypes' shapes are PR 38 `crates/libpachinko/src/clock.rs` and PR 39
     `crates/libpachinko/src/clock.rs`. `FakeClock` reads nothing from `std::time`:
     `crates/libpachinko/clippy.toml` disallows `SystemTime::now` and `Instant::now`,
     and `just effects` is the proof.
   - `lease_ids.rs`: `LeaseId`, an opaque `u128` whose high 64 bits are an epoch and low
     64 bits a serial, with `from_u128`, `as_u128`, `Display` as 32 lower-case hex digits
     and `FromStr` accepting exactly those; `ParseLeaseIdError`; the `LeaseIds` trait,
     `fn next(&self) -> LeaseId`, bounded like `Clock`; `CountingLeaseIds`, epoch 0 and a
     serial from 1, shared like `FakeClock`. PR 39 `crates/libpachinko/src/lease_ids.rs`
     line 30 `LeaseId`, line 98 `LeaseIds`, line 121 `CountingLeaseIds`.
   - `ids.rs`: newtypes `PublisherId`, `ConsumerId`, `PocketKey`, `RecordKey` over
     `String`; `Rank` over `i64`; `Payload` over `Vec<u8>`; `Position` over `u64`; each
     with `new`, `as_str` or `as_ref` and the derives invariant 3 needs. PR 39
     `crates/libpachinko/src/ids.rs` line 95 `Rank`, line 137 `Payload`, line 197
     `Position`.
   - `config.rs`: `Config` with `lease_ttl`, `max_consecutive_expiries`, `max_pull_batch`,
     `superseded_ttl` and `dead_letter_ttl`; `Default` is the specification's five values
     (S001, S002); the TTLs are `Duration`, so a negative TTL is unrepresentable and
     `LeaseTtlNonNegative`, `SupersededTtlNonNegative` and `DeadLetterTtlNonNegative` hold
     by type, which S005 pins as a `compile_fail` doctest on a negative literal, and T24's
     flag parser refuses a negative flag at the wire; `Config::check(&self) ->
     Result<(), ConfigError>`, which `Queue::new` calls, refuses
     `max_consecutive_expiries < 1` (`ExpiryLimitPositive`, S003) and `max_pull_batch < 1`
     (`PullBatchPositive`, S004). PR 39 `crates/libpachinko/src/config.rs`.
   - `error.rs`: `thiserror` enums split by operation, each `#[non_exhaustive]`:
     `JoinError { PublisherExists(PublisherId), ConsumerExists(ConsumerId) }`,
     `PublishError { UnknownPublisher(PublisherId), EqualRank { key: RecordKey, rank:
     Rank } }`, `PullError { UnknownConsumer(ConsumerId) }`, `ConfigError` with one
     variant per check. T25 adds `LeaseActionError`. PR 39
     `crates/libpachinko/src/error.rs` is the one-enum shape this splits.
   Invariant 10, "Errors are `thiserror` enums", joins the Invariants list in `AGENTS.md`
   as its tenth line; `docs/reference/agent-contract.md` line 40 says ten;
   `.agents/skills/rust-change/SKILL.md` gains one line saying an error type is a
   `thiserror` enum with a variant per refusal, and `.agents/skills/review-change/SKILL.md`
   step 2 checks ten invariants and names the tenth. The bridges are untouched. Run
   `just effects` and `just check-agents`.
5. **The store seam.** `records.rs` holds the `PocketRecords` trait, the per-pocket
   record store, from PR 38 `crates/libpachinko/src/records.rs` line 207, bounded
   `Clone + Debug + Default + Send + Sync + 'static`: `lookup(&self, &RecordKey) ->
   Option<RecordView>`; `insert_queued(&mut self, RecordKey, Rank, Payload, Position)`;
   `supersede(&mut self, &RecordKey, Rank, Payload) -> Option<(Rank, Payload)>`, the
   loser, position untouched; `hold_oldest(&mut self, usize) -> Self::Batch`, the queued
   records with the smallest positions, now held, in that order; `settle(&mut self,
   &RecordKey) -> Option<HeldRecord>`, which removes a held record; `defer(&mut self,
   &RecordKey, Position) -> Option<HeldRecord>`; `free_held(&mut self) -> usize`, which
   requeues every held and deferred record in O(outstanding) through an index of the
   outstanding keys, held and deferred, and returns how many, replacing PR 38's
   `free_all` at line 324, which walked every record;
   `record_count`, `held`, `snapshot`; an associated `type Batch: Clone + Debug + Send +
   Sync + 'static` that iterates `HeldRecord`. The held version is state-encoded as PR 39
   `crates/libpachinko/src/queue/view.rs` line 37 does: `RecordStatus::Held { rank,
   payload }` and `Deferred`, never two `Option`s beside a status. `records/memory.rs`
   holds `MemoryRecords`: a `BTreeMap<RecordKey, _>` of records plus a
   `BTreeSet<(Position, RecordKey)>` queued index and a `BTreeSet<RecordKey>` index of
   the outstanding keys. `records/contract.rs`, behind `#[cfg(feature = "testing")]`, holds
   `check_contract::<R: PocketRecords>()`, the one contract every store passes, after
   PR 38 `crates/libpachinko/src/records/contract.rs` line 22 and extended: insert then
   lookup; supersede keeps position and status and returns the loser; `hold_oldest`
   takes the smallest positions in order and never a held or deferred record; a limit
   of zero holds nothing; settle removes only a held record; defer takes the fresh
   position; `free_held` requeues held and deferred and returns the count; a second
   insert of a held key is refused, so no store can break `OneRecordPerKeyPerPocket`
   unseen; `record_count` and `held` agree with `snapshot` after every call.
   `crates/libpachinko/tests/records_contract.rs` runs it over `MemoryRecords`, and the
   feature is on under `just test`, `just coverage` and `just clippy` (`--all-features`);
   `just features` builds the powerset.
6. **The `Queue` shape.** `queue.rs` declares `Queue<R: PocketRecords, C: Clock, L:
   LeaseIds>` with `new(config: Config, clock: C, lease_ids: L) -> Result<Self,
   ConfigError>`, `config(&self) -> &Config`, and the signatures of this ticket's
   operations, each bodied enough to compile and to carry its doctest:
   - `publisher_joins(&mut self, PublisherId) -> Result<(), JoinError>` and
     `consumer_joins(&mut self, ConsumerId) -> Result<(), JoinError>`;
   - `publish(&mut self, &PublisherId, Publication) -> Result<PublishOutcome,
     PublishError>`, where `Publication { pocket_key, record_key, rank, payload }` and
     `PublishOutcome` is `Created`, `NewKey`, `Superseded` or `Stale`; `EqualRank` is
     the error, its docs and its test labelled as the open question 1 placeholder;
   - `pull(&mut self, &ConsumerId, limit: i64) -> Result<PullOutcome, PullError>`, where
     `PullOutcome` is `Batch { lease: LeaseId, pocket_key: PocketKey, expires_at:
     Timestamp, records: Vec<HeldRecord> }` or `NoPocketAvailable`, and `HeldRecord {
     key, rank, payload }` carries the held version; `expires_at` is the lease's after
     this pull, so a service reply is built from the one outcome and never from a second
     read that another command could interleave (T24);
   - `lease_view(&self, &ConsumerId) -> Result<Option<LeaseView>, PullError>`, the
     Consuming surface's `exposes`: the pocket key, `expires_at` and the held records of
     the consumer's active lease; `Ok(None)` for a joined consumer with no active lease
     and `UnknownConsumer` for one that never joined, so the service can tell the two
     apart (T24).
   The state: pockets keyed by `PocketKey`, each with its status, position,
   consecutive-expiry count and its `R`; the pocket order as a `BTreeSet<(Position,
   PocketKey)>` of queued pockets; the lease registry as `active: BTreeMap<LeaseId, _>`
   plus `by_expiry: BTreeSet<(Timestamp, LeaseId)>`, after PR 38
   `crates/libpachinko/src/queue/leases.rs` line 120 `due`, which T26 will read; an
   ended lease leaves both; one `next_position: Position` counter for pockets and
   records, the specification's `Queue` entity; the monotonic `now` clamp, `self.latest =
   self.latest.max(clock.now())`, after PR 38 `crates/libpachinko/src/queue.rs` line 199;
   the shed queue of `SupersededRecord` keyed `(expires_at, arrival)` after PR 38
   `crates/libpachinko/src/queue/shed.rs`, which `Supersede` and `DropStalePublish` feed
   and T26 drains. `Snapshot`, behind `testing`, exposes every pocket with its status,
   position and count, every record's view, every lease with its status and
   `expires_at`, the shed queue and `next_position`, enough for T28's walk to check all
   25 invariants without a second seam. `Queue` is `Clone`, and its docs say a clone
   forks the state: the two never share a pocket afterwards, while a shared `FakeClock`
   keeps ticking for both. A `compile_fail` doctest on `Queue` shows that a store type
   holding an `Rc` does not satisfy `PocketRecords` (invariant 3).
   Run `just fmt-check`, `just clippy` and `just doc` on the shape.
   **Adversarial review** (`CONVENTIONS.md` §14): the public API of steps 4 to 6, the
   trait signatures, the type bounds, the error-enum split, `LeaseId`'s encoding, the
   `Snapshot` shape and `free_held`, against this ticket's Goal and Files touched, the
   decision table in T22's Context, `docs/specs/pachinko.allium` and the T24, T25, T26
   and T28 tickets that build on them. Ask: whether a `PocketRecords` implementation can
   break `OneRecordPerKeyPerPocket` or hand out a held record twice without
   `check_contract` seeing it; whether two leases can share an `expires_at` in
   `by_expiry` and whether the key then still finds each; whether `free_held` can be O(n)
   for a store that satisfies the contract; whether any public type misses invariant 3 or
   `#[non_exhaustive]`; whether `LeaseId`'s `Display` and `FromStr` round-trip every
   `u128` and reject a 31- or 33-digit string; whether T25's four actions, T26's `tick`
   and T28's walk can be written against `Snapshot` and the signatures as they stand, or
   need a change that would break T24's wire; and whether `Queue`'s `Clone` can leave two
   queues sharing a store. Two reviewer subagents with fresh contexts run in parallel,
   one lens each, `conformance` and `breakage`, each running `codex exec -m gpt-6-astra
   -s read-only -C <worktree> -o ai_tmp/review/T23-1/findings-<lens>.md "<brief>" <
   /dev/null` (§14's command; `-o` writes only the reviewer's last message, and a
   read-only reviewer writes no file of its own); separate triage subagents check every
   finding against the tree and write `ai_tmp/review/T23-1/verdicts.md`, each finding
   valid with its fix or invalid with the reason; one writer applies the valid fixes one
   at a time, only within Files touched, and hands back any fix outside it; if a
   high-severity finding was valid, a second round reviews the fix diff and writes beside
   the first with a `-2` suffix; two rounds at most.
7. **The rules.** Fill in the ten rules against the module, reading each rule's
   `requires` as the refusal and its `ensures` as the effect:
   - `PublisherJoins` and `ConsumerJoins`: a second join of the same id is the error;
     a publish by an unjoined publisher and a pull by an unjoined consumer are the
     `Unknown*` errors, checked before anything else.
   - `PublishIntoNewPocket`: the pocket takes `next_position` and the record the next;
     `PublishNewKey`, `Supersede` and `DropStalePublish` on an existing pocket, each
     ending in the `PocketTouched` dispatch: a queued or leased pocket is untouched
     (S021, S022); the idle arm requeues at the back with count 0 and the parked arm
     does the same, both landed here, owned by T25 and T26. `Supersede` sheds the loser
     with `shed_at = now` and `expires_at = now + superseded_ttl`; `DropStalePublish`
     sheds the arrival marked stale; equal rank is `EqualRank`, not applied, not shed.
   - `PullWithinLease`: the consumer's active lease, `expires_at = now + lease_ttl`, then
     `hold_oldest(min(limit, max_pull_batch))` with a non-positive limit as zero;
     `PullClaimsFrontPocket`: no active lease, the smallest queued position, a lease from
     `lease_ids.next()` with `granted_at = now`, the pocket leased and out of the pocket
     order, then the same hold; `HoldRecord` is the store's move to `Held { rank,
     payload }`; `PullFindsNoPocket`: no queued pocket is `NoPocketAvailable`, never an
     error.
   Tests, in `crates/libpachinko/tests/queue/` after PR 38
   `crates/libpachinko/tests/queue/main.rs`: `main.rs` declares the modules, each
   `#[cfg(test)]`, and its header states the tag conventions; `fixture.rs` builds the
   queue over `MemoryRecords`, a `FakeClock` at a fixed instant and `CountingLeaseIds`,
   with helpers that join, publish and pull by literal keys and read the `Snapshot`;
   `identity.rs`, `publish.rs` and `pull.rs` carry one test per scenario this ticket
   owns, with `// scenario: S0NN` and `// obligation:` comments naming the ids from
   `just plan-spec pachinko`, and the invariant tests this ticket owns, each checking its
   invariant by name over the `Snapshot` after the operations that could break it. S019
   uses `i64::MAX`, `i64::MIN` and neighbours; S020 round-trips a payload with every byte
   value and keys with non-ASCII text; S036 reads `expires_at` from `lease_view` before
   and after a second pull with the clock advanced; S037 and S039 pull again under the
   same lease and from a second consumer; S038 publishes into three pockets and claims
   them in order with three consumers. Each test is shown to bite: the rule's guard or
   effect is broken on purpose and the test seen to fail, and the hand-back lists the
   breaks (`docs/reference/testing.md`, "Tests from the specifications"). Run
   `just test`, `just coverage` (the floor is 90% of lines over `crates/*/src/**`, so
   `Snapshot` and the fakes count) and `just doc`.
8. **The handbook.** Use the `review-docs` skill. `docs/explanation/architecture.md`
   gains "The library": the modules, the two effect traits and their fakes, the store
   seam and its contract, `Queue`'s type parameters, and the ownership rule under which
   a rule's code may land before its owner. `docs/reference/testing.md` "Tests from the
   specifications": the sentence saying the module is a placeholder goes; the `testing`
   feature, the `// obligation:` and `// scenario:` tags, the queue test layout and
   `records_contract.rs`. `docs/reference/api.md` "What the API promises", where it
   names items. `CHANGELOG.md` libpachinko Unreleased, Added. `crates/libpachinko/README.md`
   replaced: the domain, the effects, the seam, the `testing` feature, the `version()`
   example kept. Run `just check-docs`.
9. **Adversarial review** (`CONVENTIONS.md` §14): the whole diff against this ticket's
   Owns tables, Files touched and acceptance criteria, `docs/specs/pachinko.allium` and
   the decision table in T22's Context. Ask: whether every rule, invariant, guarantee and
   scenario in Owns has a test whose tags name it, and whether any tag names a scenario
   the test does not exercise; whether any owned rule's guard can be removed with every
   test still passing; whether the `PocketTouched` idle and parked arms are reachable
   here, so that a tag claims them; whether coverage counts test code or `testing`-only
   code toward the floor in a way that hides an untested branch; whether a doctest
   asserts anything or only compiles; whether `FakeClock` or anything under `src/`
   reaches `std::time` or another effect `just effects` does not probe; whether a page
   outside Files touched now says the domain is unimplemented; and whether the
   hand-back's obligation list names an id no test carries. Two reviewer subagents with
   fresh contexts run in parallel, one lens each, `conformance` and `breakage`, each
   running `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T23-2/findings-<lens>.md "<brief>" < /dev/null` (§14's command; `-o`
   writes only the reviewer's last message, and a read-only reviewer writes no file of
   its own); separate triage subagents check every finding against the tree and write
   `ai_tmp/review/T23-2/verdicts.md`, each finding valid with its fix or invalid with the
   reason; one writer applies the valid fixes one at a time, only within Files touched,
   and hands back any fix outside it; if a high-severity finding was valid, a second
   round reviews the fix diff and writes beside the first with a `-2` suffix; two rounds
   at most.
10. Use the `review-change` skill over the diff, then `project-check` for the full gate:
    `just check`, and commit on the ticket branch with the hand-back notes quoting each
    output headed by the commit it ran at. A failing recipe is fixed at its root under
    `fix-quality`. Pushing and opening a pull request are separately authorised: stop and
    ask.

## Acceptance criteria

- `just effects` is green with `clock.rs` in the tree, and `just clippy` is green with
  `crates/libpachinko/clippy.toml`'s disallowed `SystemTime::now` and `Instant::now`
  unchanged.
- `just check-agents` is green with ten invariants in `AGENTS.md`,
  `docs/reference/agent-contract.md` and the `review-change` skill agreeing on the count.
- `crates/libpachinko/tests/records_contract.rs` runs `check_contract::<MemoryRecords>()`
  and the contract has a case for each `PocketRecords` method, `free_held`'s count and
  the refused second insert included.
- Every rule, invariant, guarantee and scenario in Owns is named by a `// scenario:` or
  `// obligation:` tag on a test under `crates/libpachinko/tests/queue/` or a doctest in
  `crates/libpachinko/src/`, and `grep -rho 'scenario: S0[0-9]*' crates/libpachinko |
  sort -u` prints exactly the 31 ids in Owns.
- The hand-back lists every obligation id from `just plan-spec pachinko` this ticket's
  tests carry, and the break shown for each scenario test.
- Every public item has a doctest; `just doc` is green; `Queue`'s `compile_fail` doctest
  fails to compile for the reason its docs state.
- `just coverage` reports at or above 90% of lines over `crates/*/src/**`.
- `thiserror` appears in `[workspace.dependencies]` with its decision-0007 note and a
  full `x.y.z` version, `Cargo.lock` carries it, and `just lock-check`, `just deny` and
  `just deps-unused` are green.
- `just check-docs` and `just check` are green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding
  is fixed or handed back.

## Verification

```sh
just lock-check
just effects
just check-agents
just fmt-check
just clippy
just test
just coverage
just doc
just features
just deny
just deps-unused
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

- S005 is pinned at the type level: a negative TTL cannot be written, and the
  `compile_fail` doctest is the test. A reviewer who wants a runtime refusal in the
  library finds it has nothing to refuse; the runtime refusal is T24's flag parser.
- The idle and parked arms of `PocketTouched` land here untested and are owned by T25
  and T26; until T25 lands, those arms are dead code that coverage counts against the
  floor, and the executing agent says in the hand-back how much.
- `pachinko` does not take `thiserror` here; T24 adds `{ workspace = true }` when it
  first derives an error, so the "both crates" note in the decision-0007 comment is true
  only after T24.
- Open question 1 stays a placeholder: `EqualRank` is an error here and T27 maps it per
  row; a retried identical publish therefore gets `EqualRank`, which T27 lists.
- `Timestamp` is a `u64` of milliseconds and `Rank` an `i64`; a payload is unbounded
  here and bounded by T27's flags on the wire.
- `Queue` is `Clone` because invariant 3 asks it of every public type; nothing in the
  wave clones a live queue, and the fork semantics are documented so nobody expects a
  shared one. If a later ticket needs a shared handle, it wraps the queue in the owner
  task (T24), never in the library.
- The `queue/` submodule split is the executing agent's within `rustqual.toml`'s limits;
  the names in Files touched describe the division, not fixed file names, and the
  hand-back lists the files that exist.
