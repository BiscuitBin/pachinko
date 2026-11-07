---
id: T33
title: "Robustness: fallible lease ids, the tick receiver by value, a header-read deadline"
status: todo
depends_on: [T31]
parallel_with: [T32, T34]
branch: ticket/t33-robustness
estimated_size: M
---

# T33: Robustness: fallible lease ids, the tick receiver by value, a header-read deadline

## Context

T31 integrated the MVP wave (T22 to T31) and handed back three robustness defects as
Open points, each owned by the maintainer. This ticket fixes all three. Each is cited by
its bullet in `tickets/T31-mvp-integration.md`, "## Open points" (line 902), and none is
a product decision: the specification (`docs/specs/pachinko.allium`) names no lease id,
no tick lane and no connection deadline, so no rule changes.

**1. The lease-id exhaustion guard and the redraw loop** (T31 Open points, lines 939 to
942: "`code-review` run 2, finding 3"; disposition at line 630). Two faults:

- The owner stops the whole service at the first `Pull` after the lease-id source is
  exhausted, whether or not that pull needs a new id. `crates/pachinko/src/actor.rs`
  lines 529 to 531, in `pull`, check `lease_ids.exhausted()` before applying any pull,
  so a pull within an existing lease (`PullWithinLease`, which reuses its lease id) or a
  pull that finds no pocket (`PullFindsNoPocket`) also ends the owner with
  `ServeError::LeaseIdsExhausted` (`crates/pachinko/src/error.rs` lines 73 to 81). The
  owner holds a clone of the source only for this check (`actor.rs` lines 370 to 389,
  `start`; 417 to 440, `own`; 458 to 462, `apply`; 522 to 538, `pull`).
- `fresh_lease_id` in `crates/libpachinko/src/queue/pull.rs` lines 103 to 112 draws again
  while the id is still active. A source that repeats an active id forever spins the
  owner forever. The source's contract already says it never repeats
  (`crates/libpachinko/src/lease_ids.rs` lines 92 to 113), but `LeaseIds::next` cannot
  say "no more ids": the real source, `EpochSerialLeaseIds`
  (`crates/pachinko/src/lease_ids.rs` lines 92 to 112), returns the serial-0 id once
  exhausted and relies on the owner never calling it then. The fake,
  `CountingLeaseIds` (`lease_ids.rs` lines 126 to 137), wraps its `AtomicU64` silently
  after `u64::MAX` and would then repeat.

There is a second hazard the fix must not create. `claim_front_pocket` (`pull.rs` lines
77 to 101) pops the front pocket and marks it leased (`Pockets::claim_front`,
`crates/libpachinko/src/queue/pockets.rs` lines 176 to 181) before it draws the id. If
the draw can fail, the draw must come first, or a refused pull leaves a pocket leased
under no lease.

**The choice.** `LeaseIds::next` becomes fallible:
`fn next(&self) -> Result<LeaseId, LeaseIdError>`. `LeaseIdError` is a `thiserror` enum
(invariant 10) in `crates/libpachinko/src/lease_ids.rs`, `#[non_exhaustive]`,
`Clone + Debug + Send + Sync + 'static` (invariant 3) and `PartialEq + Eq`, as
`PullError` derives both (`error.rs` line 75), with two variants. `Exhausted` is
the source's: it cannot give an id it has not given before. `StillActive(LeaseId)` is
the queue's: the source broke its contract and returned an id that is still active. The
contract changes from "never repeats" to "never repeats, and once it cannot honour that
it returns `Err(LeaseIdError::Exhausted)` on every later call". The queue then draws
**once** per claim and checks `is_active` once. An active id is refused with
`StillActive`; it is never drawn again, so nothing spins. The draw is bounded, so the
loop is gone.

`PullError` (`crates/libpachinko/src/error.rs` lines 65 to 81) gains
`LeaseId(#[from] LeaseIdError)`. That is additive on a `#[non_exhaustive]` enum. A pull
draws an id only when it claims a pocket. A within-lease pull and a pull that finds no
pocket never call `next`, so they succeed on an exhausted source.

Why not an associated `type Error` on the trait: `PullError` would then become generic
over the source, and so would every `Queue` signature that returns it. The service's
`http.rs` conversion (`crates/pachinko/src/http.rs` lines 287 to 294) and the invariant-3
bounds would follow. A concrete library-owned error keeps `PullError` as it is.

This is a public API change to a libpachinko effect trait, and it is breaking. Every
implementor and every direct caller of `next` changes. Decisions 0003
(`docs/decisions/0003-effects-behind-traits.md`) and 0008
(`docs/decisions/0008-std-library.md`), and the effects rule in `AGENTS.md` line 51, still
govern it: the trait stays libpachinko's, the fake stays beside it, and the real
implementation stays in `pachinko`. It is **not a new kind of effect**. Randomness behind
`LeaseIds` is already recorded by decisions 0008 and 0019. 0008 line 106 asks for a
record only for a new kind, so no decision record is written. If the plan finds
otherwise, it stops and asks. `just semver` (`Justfile` lines 149 to 168) reports the
break. It is advisory before 1.0 (decision 0016;
`docs/reference/quality-gates.md` line 150). The version bump belongs to the release
process in `docs/operations/maintenance.md` (lines 95 to 115), not to this ticket.

**2. `Lifecycle::start`'s fallback tick lane** (T31 Open points, lines 952 to 955: "run
2, finding 9"; disposition at line 638). `Lifecycle` (`crates/pachinko/src/serve.rs`
lines 55 to 63) keeps the tick lane's receiver as `ticks: Option<mpsc::Receiver<Tick>>`.
`Lifecycle::start` (lines 81 to 95) takes it, and on a second call it falls back to
`actor::tick_lane().1` at line 93. That lane's sender is dropped at once, so a second
owner would never tick. T31 cited the doc sentence as `serve.rs:80`. At T31's head
(`4daf9df`) the sentence is at lines 81 to 82, "A second start gets a tick lane whose
ticker has gone". It is latent, because no second start exists.

**The choice.** The receiver stops being a field: `Lifecycle::new` returns
`(Lifecycle, mpsc::Receiver<Tick>)`, and `Lifecycle::start(&self, queue, ticks)` takes
the receiver by value. `mpsc::Receiver` is not `Clone`, so a receiver cannot be passed to
two starts, and the silent fallback lane is gone. A second start must be handed a
receiver explicitly, and only `Lifecycle::new` makes one with a live ticker.
`actor::tick_lane` stays `pub(crate)`, so the type does not stop code in the crate from
passing `actor::tick_lane().1` by hand; T33-2's `rg -n 'tick_lane\(\)\.1' crates` check
guards against that. `start` keeps `&self` rather than `self`, because `scope`
(lines 208 to 245) still destructures the lifecycle after the owner starts. Its
`ticks: _` at line 226 goes.

**3. A client trickling one byte per bound** (T31 Open points, lines 1019 to 1020:
"D-T29-6"). `Idle` (`serve.rs` lines 395 to 454) restarts `owed_since` on every byte
that arrives (`Idle::read`, lines 442 to 447). A client that sends a byte at least every
two `--request-timeout-ms` keeps a connection open until the shutdown's cut.
`docs/explanation/security-model.md` lines 138 to 142 document the gap, and so does
`docs/reference/http-api.md`, "Request timeout", lines 183 to 187.

**What exists to reach for.** `Cargo.lock` pins axum 0.8.9, hyper 1.12.0 and hyper-util
0.1.21. hyper's `http1::Builder::header_read_timeout` (hyper `src/server/conn/http1.rs`
lines 364 to 373) defaults to 30 s (line 253). It takes effect only with a `Timer` set by
`Builder::timer`. Without one, `Time::check` (hyper `src/common/time.rs` lines 73 to 89)
logs and drops the default and panics on a configured value. `axum::serve` builds
`hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())` internally (axum
`src/serve/mod.rs` line 391), sets no timer and exposes no builder. So the service has
no header-read timeout today, and none is reachable through `axum::serve`. Reaching
hyper's would mean replacing `axum::serve` with a hand-driven hyper-util accept loop, and
adding `hyper` and `hyper-util` as direct dependencies, each with a decision-0007 note
(invariant 9).

**The choice.** Use no dependency and no new flag. The deadline lives in `Idle`, which
already reads `tokio::time::Instant` in `pachinko`. The clock stays in the service, and
libpachinko is untouched by this item (invariant 2). `owed_since` becomes the instant of
the **first** byte that arrived while nothing was owed. A later byte no longer restarts
it, and only a write (`Idle::wrote`) clears it. The deadline stays two
`--request-timeout-ms` from that instant. A trickle then holds a connection for at most
two timeouts.

The trade-off, which the docs must state: the handler's own 408 fires one timeout after
the head completes. So a request whose head arrives within one timeout still gets its
reply or its 408 before the cut. A head slower than one timeout loses that guarantee: it
still gets its reply if the handler writes it within two timeouts of the first byte, and
otherwise it is cut, never with a 408, because the 408 would come after the cut.
The current sentence in `http-api.md` line 186, "A request whose head arrived in full
always gets its 408 first", no longer holds as written. The hyper-util route is the
alternative. If the lead or the T33-1 reviewers prefer it, the ticket stops and asks,
because it adds two dependencies.

The skills: `rust-change` for each change, `review-change` to review the result,
`review-docs` for the handbook lines, and `project-check` and `fix-quality` for the gate.

## Goal

`LeaseIds::next` returns `Result<LeaseId, LeaseIdError>`. A claim draws once, before the
pocket is claimed. A refused claim changes nothing, and no source can spin the queue.
After the source is exhausted, the service still answers a within-lease pull and a pull
that finds no pocket, and it stops only at a pull that needs a new id.
`Lifecycle::start` takes the tick receiver by value, and no longer falls back to a dead
lane; reusing a receiver for a second start does not compile. A connection that trickles
a head one byte at a time is cut two `--request-timeout-ms` after its first byte. Each
fix was shown to bite before it landed.

This ticket owns:

| Kind | Names |
| --- | --- |
| Library API | `lease_ids::LeaseIdError` (new); `LeaseIds::next` returning `Result`; `PullError::LeaseId` |
| Service behaviour | the owner stops only at a pull that needs an id; `Lifecycle::new` returning the receiver; the first-byte head deadline |
| Specification obligations exercised, never owned | `rule-success.PullWithinLease`, `rule-success.PullFindsNoPocket`, `rule-success.PullClaimsFrontPocket`, `rule-entity-creation.PullClaimsFrontPocket.1` |

The obligation ids are the grammar the tests already carry (`// obligation:` in
`crates/libpachinko/tests/queue/pull.rs` lines 22, 23 and 137, and
`crates/libpachinko/tests/queue/lease.rs` line 75). Each id the new tests name is
confirmed in `just plan-spec pachinko`'s output at execution time. The ticket asserts no
obligation count.

## Non-goals

- No specification change. `rg -n -i 'lease.?id' docs/specs/pachinko.allium` prints
  nothing at `4daf9df` (positive control: `rg -n 'LeaseLapses' docs/specs/pachinko.allium`
  prints lines). The module has no lease-id draw, so a refusal for an exhausted source is
  an effect failure, not a domain rule. If the plan finds that a rule must name it, it
  hands that back through `spec-change`.
- No new decision record, no new dependency and no new flag (Context). If any of them
  turns out to be needed, stop and ask.
- No version bump and no release. `just semver`'s report is quoted, and the bump belongs
  to the release process.
- No change to the tick lane's protocol, the ticker, or the owner's `biased` select order
  (T26, T29).
- No HTTP code for the new `PullError` variant. The owner stops before answering it
  (Step 5), and `http.rs`'s forced arm (`unmapped`, lines 254 to 259) still covers any
  path that reaches it.
- No change to the idle-connection bound for a connection that sends nothing (one
  timeout, `Idle::pending` line 421), nor to the shutdown's cut.
- None of T31's other Open points: run 2's findings 4, 5, 7 and 8 stay where T31 put
  them.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `crates/libpachinko/src/lease_ids.rs` | shared, own lines | `LeaseIdError`; `LeaseIds::next` returns `Result`, and its contract says so; `CountingLeaseIds` advances with a checked add and returns `Exhausted` past `u64::MAX`; doctests at lines 99 to 124 and unit tests at 139 to 185 updated, plus a unit test for the fake's exhaustion |
| `crates/libpachinko/src/error.rs` | shared, own lines | `PullError::LeaseId(#[from] LeaseIdError)` with a doctest; the unit test at lines 135 onwards covers its message |
| `crates/libpachinko/src/queue/pockets.rs` | shared, own lines | `Pockets::front(&self) -> Option<&PocketKey>`, a peek at `order`'s first entry, beside `claim_front` (line 176) |
| `crates/libpachinko/src/queue/pull.rs` | shared, own lines | `claim_front_pocket` peeks, draws once, checks `is_active` once, then claims; `fresh_lease_id`'s loop removed; `pull`'s doc and `# Errors`; the `Repeating` unit test (lines 138 to 178) now expects `StillActive`, plus a bounded-draw guard |
| `crates/libpachinko/src/lib.rs` | shared, own lines | Lines 7 to 12: the crate docs name `LeaseIdError` |
| `crates/libpachinko/README.md` | shared, own lines | Lines 12 to 15, only if the sentence needs the fallible source |
| `crates/libpachinko/tests/queue/pull.rs` | shared, own lines | An exhausted source: a within-lease pull and a no-pocket pull succeed, and a claim is refused with the snapshot unchanged |
| `crates/pachinko/src/lease_ids.rs` | shared, own lines | `EpochSerialLeaseIds::next` returns `Err(LeaseIdError::Exhausted)` after the last serial; `exhausted()` (lines 59 to 70) and the sentinel's doc (lines 93 to 96) removed; doctests at 24 to 33 and 50 to 54 and the test at 140 to 156 updated |
| `crates/pachinko/src/error.rs` | shared, own lines | `LeaseIdsExhausted`'s doc (lines 73 to 81): the owner stops at a pull that needs an id the source cannot give |
| `crates/pachinko/src/actor.rs` | shared, own lines | The owner's `lease_ids` parameter and pre-check removed from `start`, `own`, `apply` and `pull`; `pull` stops the owner on `PullError::LeaseId(_)` without answering; the docs at lines 370 to 379 and 515 to 521; tests: `run_owner`, `queue` and the two new exhaustion tests |
| `crates/pachinko/src/serve.rs` | shared, own lines | `service_queue` returns the queue alone; `Lifecycle` without `ticks`, `new` returning the receiver, `start(&self, queue, ticks)`; every caller (lines 131, 143 to 144, 180 to 183, and the tests at 619 to 620, 647 to 648, 744 to 745, 821 to 823, 869 to 877, 918 to 922, 979 to 980); `scope`'s destructuring (line 226); `Idle` and its doc (lines 395 to 454); the trickle test |
| `crates/pachinko/src/http.rs` | shared, own lines | The test helper `unanswered_router` (lines 699 to 715) calls `actor::start` without the source |
| `docs/explanation/architecture.md` | shared, own lines | Lines 150 to 157 (the `LeaseIds` paragraph: fallible); lines 243 to 248 (the owner stops at a pull that needs an id) |
| `docs/reference/api.md` | shared, own lines | Lines 70 to 77: `LeaseIdError` among the refusal enums |
| `docs/reference/http-api.md` | shared, own lines | "Request timeout", lines 183 to 187: the head deadline from the first byte and the one-timeout guarantee |
| `docs/explanation/security-model.md` | shared, own lines | Lines 138 to 142: the trickle is bounded, and the gap sentence is retired |
| `docs/reference/configuration.md` | shared, own lines | Line 46, the `--request-timeout-ms` row: it also bounds a request head, two timeouts from its first byte |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, "Changed": the breaking `LeaseIds::next` change; pachinko Unreleased: the exhaustion stop and the head deadline |
| `tickets/T33-robustness.md` | ticket | `status:` and hand-back notes |

T32 and T34 run in parallel. T32 may edit `crates/libpachinko/src/queue/pockets.rs`,
`docs/explanation/architecture.md`, `docs/reference/http-api.md` and `CHANGELOG.md`; T34
may edit `docs/reference/configuration.md` and `CHANGELOG.md`. Neither owns any other
path in this table, so a sibling's change to `actor.rs`, `serve.rs`, `http.rs`,
`lease_ids.rs`, `error.rs` or any other path here is not an overlap but an unplanned
change: stop and report it rather than merge. In each shared file named here this ticket
edits only its own lines, and on a conflict in one of them it keeps both tickets' lines
(Step 10). Line numbers are as of `4daf9df`.
A file that moved by the time this ticket runs is located by the quoted text, and the
hand-back records the new lines.

## Steps

1. Create the worktree from `main` after T31's pull request has merged (`README.md`,
   "How to pick up a ticket"; `CONVENTIONS.md` §11, "Worktrees and branches"), and run
   `just initialize` there (network, authorised by this step). Or, in a clean worktree
   already initialized on `main` whose branch carries nothing, create the branch there
   and skip `just initialize`. Run `just check` on the untouched worktree and quote its
   last line as the baseline, headed by its commit. A red baseline means `main` is
   broken: stop and hand it back as a `main` pull request.
2. **The plan.** Use the `plan-change` skill. Read every line Files touched cites. Read
   the four pull rules in `docs/specs/pachinko.allium` (`PullWithinLease`,
   `PullClaimsFrontPocket`, `HoldRecord`, `PullFindsNoPocket`). Run
   `just plan-spec pachinko` and confirm that each obligation id in the Goal table is
   printed. Run the Non-goals' spec search with its positive control. List every
   implementor of `LeaseIds` and every direct caller of `next` with
   `rg -n 'impl LeaseIds|\.next\(\)' crates` (at `4daf9df`: the three impls at
   `crates/libpachinko/src/lease_ids.rs:132`,
   `crates/libpachinko/src/queue/pull.rs:142` and
   `crates/pachinko/src/lease_ids.rs:92`, and the doctests and tests in both
   `lease_ids.rs` files). List every caller of `Lifecycle::new`, `Lifecycle::start`,
   `actor::start` and `service_queue` with
   `rg -n 'Lifecycle::new|lifecycle\.start|actor::start\(|service_queue\(' crates`.
   Confirm the three Context findings still reproduce on the tree, by reading the cited
   lines. If any was fixed by another ticket, record that and drop it from this one.
3. **The red steps, before any fix.** Use the `rust-change` skill. These run at the
   untouched base. Each is a scratch test that is never committed in its red form. The
   hand-back quotes each failure, headed by its commit. After its failure is quoted,
   each red test is taken out of the worktree and kept under `ai_tmp/` (never
   `#[ignore]`d). It is restored, unchanged, in the step that turns it green: the spin
   test in Step 4, the two actor tests in Step 5, the trickle test in Step 7. The
   dead-lane test is never restored; Step 6 reruns it as a scratch copy. No step's
   `just test` runs with a red test in the tree.
   - Item 1, the spin: in `pull.rs`'s test module, a source whose `next` always returns
     `LeaseId::from_u128(1)`, counts its draws in an `Arc<AtomicUsize>` and panics once
     it has been drawn more than twice. Pockets `a` and `b` are queued, and one consumer
     claims `a` under id 1. A second consumer's pull runs directly on the test thread,
     with no spawned thread and no `recv_timeout`. At the base the source panics,
     because `fresh_lease_id` keeps drawing, and that panic is the red failure.
   - Item 1, the needless stop: two tests in `actor.rs` over `run_owner` with
     `EpochSerialLeaseIds::starting_at(u64::MAX)`. In the first, `c1` claims pocket `a`
     (the last id) and then pulls again, within its lease. In the second, `c1` claims the
     only pocket and `c2` then pulls and finds none. Each expects the second pull to be
     answered: `Batch` under the same lease, and `NoPocketAvailable`. At the base both
     fail: the owner ends with `ServeError::LeaseIdsExhausted` and the second answer is
     dropped. These two tests are committed once green, unchanged.
   - Item 2, the dead lane: in `serve.rs`'s tests, a config with `--lease-ttl-ms 0`
     and one `Lifecycle` started twice; the first owner is never spawned, the second
     is. `actor::ticker` runs over the lifecycle's `lane` with a 1 ms sweep. Through the
     second owner's handle: a publisher and a consumer join, one record is published and
     pulled, and the test polls `lease_view` until it reports no lease, with a 5-second
     deadline that fails the test. At the base the deadline is reached, because the
     second owner's lane has no sender, so its lease never lapses.
   - Item 3, the trickle, the test
     `a_trickling_head_is_cut_at_the_header_read_deadline` in `serve.rs`'s tests: a
     `serve_until_closed`-style helper (`serve.rs` lines 809 to 832) at
     `--request-timeout-ms 100`. The client sends `G`, then one more byte of a
     request line every 50 ms for 3 seconds, and records whether and when the server
     closed. The test asserts the close came at least 190 ms and under 1500 ms after the
     first byte. At the base the connection stays open for the whole trickle, and the
     assertion fails. It is committed once green, unchanged.
4. **The library API.** Use the `rust-change` skill. In
   `crates/libpachinko/src/lease_ids.rs`, write `LeaseIdError` and the fallible `next`
   with the contract in Context. `CountingLeaseIds` advances with
   `fetch_update`/`checked_add` and returns `Err(Exhausted)` once its counter cannot
   advance, and keeps doing so. A unit test sets the private counter near `u64::MAX`
   inside the module's `#[cfg(test)]` and shows the last id issued once, then
   `Exhausted` twice. In `error.rs`, write `PullError::LeaseId(#[from] LeaseIdError)`,
   with a doctest and the message test. In `pockets.rs`, write `Pockets::front`. In
   `pull.rs`, `claim_front_pocket` returns `Result<PullOutcome, PullError>` and runs in
   this order:
   1. `front()`, giving `NoPocketAvailable` when it is `None`, with no draw.
   2. One `lease_ids.next()?`.
   3. `is_active(id)`, giving `Err(LeaseIdError::StillActive(id))` with no second draw.
   4. Only then `claim_front`, the grant and the hold.

   A refused claim mutates nothing. `fresh_lease_id` goes. The `Repeating` test expects
   `Err(PullError::LeaseId(LeaseIdError::StillActive(LeaseId::from_u128(1))))` for the
   second claim, and `a` is still the only pocket leased. The step-3 spin test, restored
   and now green, stays as the bounded-draw guard: `c2`'s pull returns
   `Err(PullError::LeaseId(LeaseIdError::StillActive(LeaseId::from_u128(1))))`, and the
   source was drawn exactly twice. In `crates/libpachinko/tests/queue/pull.rs`,
   one test with a source exhausted after its first id covers the claim path: a
   within-lease pull succeeds (`// obligation: rule-success.PullWithinLease`), and a
   claim of a second queued pocket is refused with `LeaseId(Exhausted)` and the
   snapshot equals the one taken before it. A separate setup covers the no-pocket path:
   a second queue whose source is exhausted from the start, with nothing published,
   answers a pull with `NoPocketAvailable`, which shows the no-pocket path never draws
   (`// obligation: rule-success.PullFindsNoPocket`).
   Update the crate docs, every doctest that calls `next`, and `pull`'s `# Errors`.
   The service's source moves to the new signature in this step, so the workspace
   builds: in `crates/pachinko/src/lease_ids.rs`, `EpochSerialLeaseIds::next` returns
   `Result<LeaseId, LeaseIdError>`, with `Err(LeaseIdError::Exhausted)` once the serial
   has passed `u64::MAX`. The last serial is still issued once. Its doctests (lines 24
   to 33 and 50 to 54) and its tests (lines 125 and 140 to 156) move to the fallible
   signature, and the test at lines 140 to 156 asserts `Exhausted` after the last
   serial. `exhausted()` stays until Step 5, because `actor.rs` line 529 still calls
   it. Run `just fmt-check`, `just clippy`, `just test`, `just doc` and `just effects`;
   the workspace must compile at this checkpoint.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint T33-1, after the library
   API and before the service follows it. Use the diff of Step 4, which includes
   `crates/pachinko/src/lease_ids.rs`, against the Context
   choice, Files touched and `AGENTS.md` invariants 2, 3, 9 and 10. Ask:
   - whether any path through `pull` mutates a pocket, a lease or a consumer before a
     draw that can fail;
   - whether a within-lease pull or a no-pocket pull can still reach `next`;
   - whether any loop over `next` remains;
   - whether `CountingLeaseIds` or `EpochSerialLeaseIds` can return an id it already
     returned, wrap included;
   - whether `LeaseIdError` meets invariant 3 and is `#[non_exhaustive]`;
   - whether an associated error type or a different variant split would serve a caller
     better;
   - whether this is in truth a new kind of effect that needs a decision record
     (decision 0008, line 106);
   - whether the spec needs a rule for the refusal.

   Two reviewer subagents with fresh contexts run in parallel, one lens each,
   `conformance` and `breakage`. Each runs on the harness §14 selects: the one the
   executing agent is not, when it is on `PATH`, otherwise its own. Confirm each command
   with `--help` on the day. From Claude Code, run
   `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T33-1/findings-<lens>.md "<brief>" < /dev/null`. From Codex, run
   `claude -p --model claude-opus-5-5 --permission-mode plan "<brief>" >
   ai_tmp/review/T33-1/findings-<lens>.md < /dev/null` from the worktree (§14's
   commands; `-o` writes only the reviewer's last message, and a read-only reviewer
   writes no file of its own). Separate triage subagents check every finding against
   the tree and write `ai_tmp/review/T33-1/verdicts.md`, marking each finding valid
   with its fix or invalid with the reason. One writer applies the valid fixes one at a
   time, only within Files touched, and hands back any fix outside it. If a
   high-severity finding was valid, a second round reviews the fix diff and writes
   beside the first with a `-2` suffix. Two rounds at most. A valid finding that the API
   should change in shape is applied here, before Step 5 builds on it.
5. **The service follows.** Use the `rust-change` skill. `EpochSerialLeaseIds::next`
   is already fallible (Step 4). In `crates/pachinko/src/lease_ids.rs`, `exhausted()`
   and the serial-0 sentence go. In `actor.rs`, the owner no longer holds a source:
   `start`, `own`, `apply` and `pull` lose `lease_ids`. `pull` applies `queue.pull`. On
   `Err(PullError::LeaseId(_))` it ends the owner with `ServeError::LeaseIdsExhausted`
   without answering the reply, as it does today, so `http.rs`'s forced arm never turns
   it into a 503. On every other outcome it answers. Keep
   `an_exhausted_source_grants_its_last_id_then_stops_the_owner` (lines 718 to 731) as
   the guard that a claim still stops it. Restore the two step-3 actor tests; they now
   pass. In `serve.rs`, `service_queue` returns the queue alone. Update
   `unanswered_router` in `http.rs`. Update `error.rs`'s `LeaseIdsExhausted` doc. Run
   `just test` and `just effects`.
6. **The tick receiver by value.** Use the `rust-change` skill. In `serve.rs`: remove
   `ticks` from `Lifecycle`; `Lifecycle::new` returns `(Self, mpsc::Receiver<Tick>)`;
   `start(&self, queue, ticks: mpsc::Receiver<Tick>)` passes it to `actor::start`; drop
   the line-82 sentence and the line-93 fallback; and remove `ticks: _` from `scope`.
   Update every caller listed in Files touched. The test at lines 869 to 877 already
   takes the receiver by hand and now takes it from `new`. The step-3 dead-lane test
   cannot be written against the new API. Rerun it as a scratch copy that passes the
   same `ticks` binding to two `start` calls, and quote the compiler's error for the
   second `start` as the green evidence that a receiver cannot be reused. Only E0382,
   "use of moved value", counts as green; it does not show that a second lifecycle is
   required. The test is then deleted, never committed, and the hand-back says so.
   Run `just clippy` and `just test`.
7. **The head deadline.** Use the `rust-change` skill. In `serve.rs`'s `Idle`,
   `read(true)` sets `owed_since` only when it is `None`, and `wrote()` still clears it.
   Rewrite the field's doc (lines 403 to 410) and `pending`'s message (lines 430 to 436)
   to say "of the first byte". Keep the existing tests
   `a_connection_that_sends_nothing_is_closed_within_the_request_timeout` and
   `a_connection_that_stalls_inside_a_head_is_closed_within_two_request_timeouts`
   (lines 834 to 852) green, and keep every test that writes a reply after a body
   (`start_a_request` and the 100-continue path) green. Restore the step-3 trickle test
   here; it now passes. The deadline is the assertion, with a wide upper margin: never
   assert a tight elapsed upper bound. Run `just test` and `just coverage`.
8. **The handbook.** Use the `review-docs` skill. Edit the lines in Files touched:
   - `http-api.md`, "Request timeout": a connection still owed a reply is closed two
     timeouts after the first byte of the request, however it trickles, and a head that
     arrives within one timeout always gets its reply or its 408 first;
   - `security-model.md`: the trickle is bounded at two timeouts per request, and the
     "no trickle or slowloris bound exists" sentence goes;
   - `configuration.md` line 46: the `--request-timeout-ms` row adds the head bound;
   - `architecture.md`: the fallible `LeaseIds`, and the owner stopping at a pull that
     needs an id;
   - `api.md`: `LeaseIdError`;
   - `CHANGELOG.md`: both Unreleased sections, the libpachinko entry under "Changed",
     naming the breaking signature.

   Then search for stale sentences. The search must print nothing afterwards. Its
   positive control is the same search at `4daf9df`, which prints five lines:
   `security-model.md:141` and `:142`, `http-api.md:185`, and `serve.rs:82` and `:432`:

   ```sh
   rg -n -i 'no trickle|slowloris bound exists|after its last byte|of the last byte|second start gets' docs crates
   ```

   Run `just check-docs`.
9. **Adversarial review** (`CONVENTIONS.md` §14), checkpoint T33-2, over the whole diff
   (`git diff origin/main...HEAD` plus the uncommitted changes) before the final
   commit. Review it against the Goal, Files touched, the acceptance criteria and the
   red steps' quotes. Ask:
   - whether each red step would still fail with its fix reverted, in particular
     whether the trickle test passes on the base when the trickle interval exceeds two
     timeouts, and whether the actor tests pass with the pre-check restored;
   - whether a refused claim can leave state behind;
   - whether the owner can answer a `LeaseId` refusal as 503 instead of stopping;
   - whether any caller still builds a dead tick lane (`rg -n 'tick_lane\(\)\.1'
     crates`, where the test helper `no_ticks` in `actor.rs` and `unanswered_router` in
     `http.rs` remain by design, because they test an owner with no ticker);
   - whether the head deadline cuts a legitimate request whose head completed within
     one timeout, or a slow body after a `100 Continue`;
   - whether a keep-alive connection's second request inherits the first request's
     instant;
   - whether `just semver`'s findings are exactly the intended break;
   - whether any file outside Files touched changed.

   Two reviewer subagents, `conformance` and `breakage`, each run on the harness §14
   selects, as at T33-1. From Claude Code, run
   `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T33-2/findings-<lens>.md "<brief>" < /dev/null`. From Codex, run
   `claude -p --model claude-opus-5-5 --permission-mode plan "<brief>" >
   ai_tmp/review/T33-2/findings-<lens>.md < /dev/null` from the worktree. Triage, a
   single writer, the scope guard and at most two rounds follow as at T33-1, with
   verdicts in `ai_tmp/review/T33-2/verdicts.md`. Rerun the Verification block after
   the fixes.
10. **Review, merge and gate.** Use the `review-change` skill over the diff, then the
    `project-check` skill: `just check`, with `fix-quality` for any failing recipe. Run
    `git fetch origin` (network, authorised by this step) and merge `origin/main`
    (`CONVENTIONS.md` §11). T32 or T34 may have merged since this branch was cut, so a
    conflict in a file the Files touched note names as shared with T32 or T34 keeps both
    tickets' lines. `Cargo.lock` is never merged by
    hand: if it conflicts, `just lock` (network) regenerates it, and that run is a
    separate stop: ask first. Rerun the Verification block at the merge commit and
    requote it. Commit on the ticket branch with the hand-back notes, each quoted output
    headed by the commit it ran at. **Pushing and opening the pull request are
    separately authorised: stop and ask.**

## Acceptance criteria

- `LeaseIds::next` returns `Result<LeaseId, LeaseIdError>`. `LeaseIdError` is a
  `thiserror` enum, `#[non_exhaustive]`, `Clone`, `Debug`, `PartialEq`, `Eq` and
  `Send + Sync + 'static`,
  with a doctest on each public item, and `PullError::LeaseId` converts from it.
- No loop over `next` remains in libpachinko:
  `rg -n 'fn fresh_lease_id|while .*is_active' crates/libpachinko/src/queue/pull.rs` prints
  nothing, and a source that always repeats an active id gets a `StillActive`
  refusal after exactly two draws (the bounded-draw guard). The hand-back quotes that
  guard's source panicking at the base.
- A claim refused by the source or by `StillActive` leaves the snapshot equal to the one
  taken before it. The draw precedes `claim_front` in `pull.rs`.
- On an exhausted source, the library answers a within-lease pull with a batch and a
  pull with no queued pocket with `NoPocketAvailable`. The service's owner answers both
  and stops only at a pull that needs a new id. The hand-back quotes both actor tests
  failing at the base.
- `CountingLeaseIds` and `EpochSerialLeaseIds` each issue their last id once and then
  return `Exhausted` on every later call. `EpochSerialLeaseIds::exhausted` no longer
  exists.
- `Lifecycle` has no `ticks` field, and `Lifecycle::start` takes the receiver by value.
  The hand-back quotes the dead-lane test failing at the base and the compiler refusing a
  second start with the same receiver (E0382) afterwards.
- A connection trickling a head one byte per half-timeout is closed between two timeouts
  and its upper margin after its first byte, and the existing idle and stalled-head
  tests stay green. The hand-back quotes the trickle test,
  `a_trickling_head_is_cut_at_the_header_read_deadline`, failing at the base.
- No dependency was added:
  `git diff origin/main...HEAD -- Cargo.toml crates/*/Cargo.toml Cargo.lock` prints
  nothing, with its positive control being the same command, from the same
  `origin/main` base, against a commit that changed a manifest.
- No decision record and no specification line changed:
  `git diff origin/main...HEAD -- docs/decisions docs/specs` prints nothing.
- `just effects` passes, and `just coverage` reports at or above 90% of lines over
  `crates/*/src/**`.
- `just semver` is quoted in full. Every incompatibility it reports concerns
  `LeaseIds::next` (and, if reported, `CountingLeaseIds`). The CHANGELOG's libpachinko
  "Changed" entry names the break.
- Step 8's stale-sentence search prints nothing after its positive control, and
  `just check-docs` is green.
- The adversarial reviews T33-1 and T33-2 ran (`CONVENTIONS.md` §14), and every valid
  finding is fixed or handed back.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing, at the commit that merges `origin/main`.

## Verification

Every command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
line that passes by printing nothing is preceded by its positive control.
`just semver` is expected to exit non-zero on the intended break. The hand-back quotes
its lints, and any lint outside the intended change is a finding.

```sh
just plan-spec pachinko; echo "exit $?"
rg -n 'LeaseLapses' docs/specs/pachinko.allium; echo "exit $? (positive control, must match)"
rg -n -i 'lease.?id' docs/specs/pachinko.allium; echo "exit $? (must be 1)"
just fmt-check; echo "exit $?"
just clippy; echo "exit $?"
just test; echo "exit $?"
just doc; echo "exit $?"
just effects; echo "exit $?"
just coverage; echo "exit $?"
just semver; echo "exit $?"
rg -n 'impl LeaseIds' crates; echo "exit $? (positive control, at least three impls)"
rg -n 'fn fresh_lease_id|while .*is_active' crates/libpachinko/src/queue/pull.rs; echo "exit $? (must be 1)"
rg -n 'ticks: Option|ticks\.take\(\)' crates/pachinko/src/serve.rs; echo "exit $? (must be 1)"
rg -n 'fn a_trickling_head_is_cut_at_the_header_read_deadline' crates/pachinko/src/serve.rs; echo "exit $? (must be 0)"
# positive control: the same diff over the whole branch prints the files T33 changed
git diff --stat origin/main...HEAD; echo "exit $? (must list crates/ paths)"
git diff --stat origin/main...HEAD -- Cargo.toml crates/libpachinko/Cargo.toml crates/pachinko/Cargo.toml Cargo.lock docs/decisions docs/specs; echo "exit $? (must print nothing)"
just check-docs; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Hand-back notes

### Baseline

### The plan (step 2)

### Shown to bite (step 3, the red steps)

### The library API (step 4)

### T33-1: the library API

### The service follows (step 5)

### The tick receiver by value (step 6)

### The head deadline (step 7)

### The handbook (step 8)

### T33-2: the whole diff

### Step 10: `review-change`, merge and gate

## Open points

- Whether the lead prefers hyper's own `header_read_timeout`, through a hand-driven
  hyper-util accept loop with `hyper` and `hyper-util` as direct dependencies, to the
  in-crate deadline. That route needs two decision-0007 notes and is not taken here.
- A head slower than one `--request-timeout-ms` is no longer guaranteed a reply: it is
  cut, without a 408, unless the handler replies within two timeouts of the first byte.
  Whether a
  separate head-read flag is wanted, beside T31's D-T29-2 (the idle and handler
  timeouts share one flag), is the maintainer's.
- The libpachinko version bump for the breaking `LeaseIds::next` change belongs to the
  release process (`docs/operations/maintenance.md`), not this ticket.
