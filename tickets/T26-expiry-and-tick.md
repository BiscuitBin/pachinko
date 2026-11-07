---
id: T26
title: "Lease expiry, decay and the tick lane"
status: todo
depends_on: [T25]
parallel_with: []
branch: ticket/t26-expiry-and-tick
estimated_size: M
---

# T26: Lease expiry, decay and the tick lane

## Context

After T25 a consumer can pull, ack, nack, renew and release, and a lease that is never
renewed or released lasts forever: nothing reads the clock against `expires_at`, no
pocket ever parks, and the superseded records T23 sheds and the dead letters T25
creates never decay. This ticket lands the clock's side of the module: the lapse chain
(`LeaseLapses` and the three rules that process `LeaseLapsed`), parking after
consecutive expiries and the parked arm of `PocketTouched`, the two decays, and the
service's sweep that drives them.

The design is the decision table in T22's Context and decision 0019. One owner task
applies every command; the sweep's `Tick` is a command the same owner applies, so a
lapse and a consumer's action are totally ordered and the module's expiry comment holds
by construction: an action processed first moves `expires_at` or ends the lease and the
lapse stops at its guards; a lapse processed first ends the lease and the action is
refused. That is OQ8 closed with branch A, which T22 wrote into the module's lapse
section and `ExpiryRecoversWithinTtl`: a late pull, ack, nack or renew succeeds and
extends the lease, a late release succeeds by ending it and freeing its records, neither
is refused for being late, and recovery after a dead consumer takes `lease_ttl`, plus
up to one sweep, plus the application of the tick in progress and of the command the
owner is applying; a tick samples `now` once, so a lease expiring during a long tick
waits for the next.
0019 also records why the tick travels on its own lane rather than in the client FIFO:
PR 38's `Tick` queued behind a batch backlog and its lapse lag grew to seconds under
load, so here the owner polls a separate tick channel first.

The prototypes this ticket reads are heads of unmerged branches; a fresh worktree needs
`git fetch origin` (network, authorised by this step) before `git show` can read them.
Never check a prototype branch out into the ticket's worktree; read with
`git show <head>:<path>`.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 38, the Flight service prototype | `tickets/t20-pachinko-prototype-complete` | `cb77f07` | `git show cb77f07:<path>` |
| PR 39, the HTTP service prototype | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` | `git show cf1a6b2:<path>` |
| PR 36, the worker-pool spike | `tickets/t20-worker-pool-spike-complete` | `0fa614e` | `git show 0fa614e:<path>` |

What each prototype contributes: PR 38 `crates/libpachinko/src/queue/expiry.rs` line
36 (`tick`) and line 54 (`lapse`), `crates/libpachinko/src/queue/leases.rs` line 120
(`due`, the `by_expiry` prefix), `crates/libpachinko/src/queue/shed.rs` line 43 (`decay`
by `BTreeMap::split_off`), `crates/libpachinko/src/queue/pockets.rs` line 250 (the
pocket's lapse arm), and `crates/pachinko/src/actor.rs` line 105 (`Tick`) and line 281
(the ticker in `start`, which this ticket moves onto its own lane); PR 39
`crates/pachinko/src/serve.rs` line 286 (`sweep`, the periodic ticker, whose interval
this ticket replaces with a sleep after each tick's reply); PR 36
`spikes/worker-pool/tests/invariants.rs` line 417
(`a_lapse_and_a_late_renewal_are_ordered_by_the_pocket_machine_either_way`, the
two-order technique) and line 386 (the fencing-after-lapse test). PR 39's
`docs/reference/http-api.md` describes the lapse window as an open question; this ticket
describes it as decided.

## Goal

`Queue::tick` processes everything the clock has made due, in O(due) rather than O(n),
and reports it; a lease lapses exactly at `expires_at` and once; a pocket parks on its
`max_consecutive_expiries`-th consecutive lapse and a publish revives it; superseded
records and dead letters decay at their `expires_at`; the service sweeps every
`--sweep-ms` on a tick lane the owner polls first; the branch-A order is pinned in both
directions; and every scenario below is pinned by a tagged test.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (7) | `ReviveParkedPocket`, `LeaseLapses`, `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`, `LeaseExpiryIdlesEmptyPocket`, `SupersededRecordDecays`, `DeadLetterDecays` |
| Invariant (1) | `ExpiriesWithinLimit` |
| Guarantee (1) | `ExpiryRecoversWithinTtl`, in T22's branch-A wording |
| Flag (1) | `--sweep-ms` (default 1000, minimum 1) |

The scenarios this ticket owns, each pinned by a library test carrying `// scenario:`:

| Id | Scenario |
| --- | --- |
| S006 | A zero `lease_ttl` lapses at the instant of grant |
| S007 | Zero superseded and dead-letter TTLs decay at once |
| S024 | A supersede or a stale publish revives a parked pocket |
| S025 | An equal-rank publish does not revive a parked pocket |
| S026 | Only a publish revives a parked pocket (OQ7 as the rules stand) |
| S034 | Idle, parked and leased pockets are never claimed |
| S048 | A supersede while the key is held defers it on nack, and the lapse frees it |
| S056 | A release resets the consecutive-expiry count |
| S058 | A released lease never lapses afterwards |
| S059 | A lease lapses exactly at `expires_at` |
| S060 | A lapse keeps the pocket at the front and requeues its records |
| S061 | A lapse with no records idles the pocket |
| S062 | Three consecutive lapses park the pocket, and a publish revives it |
| S063 | At the threshold an empty pocket parks rather than idling |
| S064 | Progress under a lease does not reset the expiry count |
| S065 | A lapse fires once per lease |
| S066 | A renew or release after `expires_at` but before the lapse is processed succeeds (branch A) |
| S067 | An ack or pull after `expires_at` but before the lapse is processed succeeds (branch A) |
| S069 | After a lapse, the old lease is fenced even when the key has a newer rank |
| S073 | Superseded entries decay at `shed_at` plus `superseded_ttl` |
| S074 | Dead letters decay at `nacked_at` plus `dead_letter_ttl` |
| S075 | Pockets and records form one strict, deterministic order |

## Non-goals

- No change to the four actions: T25's guards stay registry-only, which is what makes
  branch A hold. A finding that an action needs an `expires_at` guard is a specification
  question and is handed back.
- No per-request logging, log-level flag, readiness or shutdown bound (T29): lapses log
  at `info` as a count per tick under T24's subscriber, and the ticker stops the way
  T24's tasks stop.
- No measurement of lapse lag under load: T30's probe. This ticket proves the bound with
  short TTLs on loopback, not with numbers.
- No answer to open question 7: S026 pins that only a publish revives a parked pocket,
  as the rules stand, and says so.
- No suspend-aware clock: the monotonic clamp is the clock trade-off 0019 records, and
  this ticket pins what the clamp does, not what a suspended host does.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `crates/libpachinko/src/queue/expiry.rs` | new | `tick` and `TickReport`; the lapse arm selecting among the three expiry rules; unit tests under `#[cfg(test)]` |
| `crates/libpachinko/src/queue.rs` | shared, own lines | The `mod expiry` line; the parked arm of `PocketTouched` (`ReviveParkedPocket`); the re-export of `TickReport` |
| `crates/libpachinko/src/queue/leases.rs` | shared, own lines | `due(now)`: the `by_expiry` prefix at or before `now`, earliest first |
| `crates/libpachinko/src/queue/shed.rs` | shared, own lines | `decay(now)`: `split_off` at `(now, u64::MAX)` on both queues, returning the counts |
| `crates/libpachinko/src/lib.rs` | shared, own lines | The crate docs name the tick |
| `crates/libpachinko/tests/queue/main.rs` | shared, own lines | The `expiry`, `decay` and `counting` module lines |
| `crates/libpachinko/tests/queue/fixture.rs` | shared, own lines | A short-TTL config helper; a snapshot order check |
| `crates/libpachinko/tests/queue/counting.rs` | new | A `PocketRecords` wrapper that delegates to `MemoryRecords` and counts each call |
| `crates/libpachinko/tests/queue/expiry.rs` | new | The lapse, parking, revive, fencing, clamp, O(due) and both-order scenarios |
| `crates/libpachinko/tests/queue/decay.rs` | new | S007, S073, S074 |
| `crates/pachinko/src/actor.rs` | shared, own lines | The tick lane: its channel, the `biased` select, the `Tick` arm and its reply, the ticker task, the both-lanes-ready test |
| `crates/pachinko/src/config.rs` | shared, own lines | `--sweep-ms` |
| `crates/pachinko/src/serve.rs` | shared, own lines | The ticker started with the owner, stopped with it |
| `crates/pachinko/tests/http.rs` | shared, own lines | The wire lapse test |
| `crates/pachinko/tests/binary.rs` | shared, own lines | `--sweep-ms 0` and a negative value are usage errors |
| `docs/reference/http-api.md` | shared, own lines | The lapse window paragraph, as decided |
| `docs/reference/configuration.md` | shared, own lines | `--sweep-ms` in "Service flags" |
| `docs/explanation/architecture.md` | shared, own lines | "The service": the tick lane paragraph |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Added; pachinko Unreleased, Added |
| `tickets/T26-expiry-and-tick.md` | ticket | `status:` and hand-back notes |

T27 and T29 may still be landing when this ticket starts, and both edit `actor.rs`,
`config.rs`, `serve.rs`, `tests/http.rs`, `tests/binary.rs` and the handbook pages above;
this ticket edits only its own lines in each.

## Steps

1. Create the worktree from `main` after T25 has merged (`README.md`, "How to pick up a
   ticket") and run `just initialize` there (network, authorised by this step), or, in a
   clean worktree already initialized on `main` whose branch carries nothing, create the
   branch there and skip `just initialize`. Run `just check` on the untouched worktree
   and quote its last line as the baseline. Run `git fetch origin` (network, authorised
   by this step) so `git show` can read the three prototype heads.
2. **The plan.** Use the `plan-change` skill. Read the seven rules, the lapse section's
   comment, `ExpiriesWithinLimit` and `ExpiryRecoversWithinTtl` in
   `docs/specs/pachinko.allium`; run `just plan-spec pachinko` for the obligation ids.
   Read T23's lease registry and shed queue, T25's `free_held` path and registry `end`,
   and T24's owner loop and `serve` before designing against them.
3. **The library tick.** Use the `rust-change` skill. In `expiry.rs`:
   - `tick(&mut self) -> TickReport`, with `TickReport { lapsed: Vec<Lease>,
     decayed_superseded: usize, decayed_dead_letters: usize }`, `#[non_exhaustive]`,
     invariant 3, a doctest, and behind `#[cfg(feature = "testing")]` two more fields:
     `leases_examined`, the `by_expiry` entries `due` read, the first entry past `now`
     that stops the walk included, and `shed_examined`, the shed entries the decay's own
     code visited, which is zero when `split_off` does the work (the split is std's, so
     only the module's own iteration is counted). `now` is the queue's clamped reading,
     taken once per tick.
   - Lapses: `due(now)` walks the `by_expiry` prefix with `expires_at <= now`, earliest
     first, and stops at the first later entry, so the cost is the number due, never the
     number active. Each due lease ends as expired with `ended_at = now`, leaves both
     registry indexes (so a second tick finds nothing: S065), and its consumer's active
     lease is cleared. The pocket then, in this order: `consecutive_expiries` rises by
     one; held and deferred records are freed by T25's `free_held`, each back to queued
     at its own position (`RequeueHeldRecord`, `RequeueDeferredRecord`); if the count
     has reached `max_consecutive_expiries` the pocket parks
     (`LeaseExpiryParksPocket`, checked before the records are counted, so an empty
     pocket at the threshold parks: S063); else with records it stays queued with its
     position untouched, so it keeps the front (`LeaseExpiryKeepsPocketAtFront`); else
     it goes idle (`LeaseExpiryIdlesEmptyPocket`). `ExpiriesWithinLimit` holds because
     the parked arm is taken exactly when the count reaches the limit and a parked pocket
     has no lease to lapse again.
   - Decay: `shed.decay(now)` takes the prefix at or before `(now, u64::MAX)` of each
     shed queue by `split_off`, O(due) plus the tree split, and returns the two counts
     (`SupersededRecordDecays`, `DeadLetterDecays`).
   - `ReviveParkedPocket`: the parked arm of `PocketTouched` requeues the pocket at the
     back with count 0; an equal-rank publish is refused before the dispatch, so it
     touches nothing (S025).
   - A pull skips idle, parked and leased pockets because only queued pockets are in the
     order (S034); this ticket adds the test, not the code.
4. **The library tests.** Use the `rust-change` skill. The tests in `tests/queue/expiry.rs`
   and `tests/queue/decay.rs` pin the 22 scenarios above, each tagged `// scenario:` and
   `// obligation:`. The designs that need stating:
   - S059: `lease_ttl` 10 ms; pull at 0; tick with the clock at 9 lapses nothing; at 10
     it lapses one, with `ended_at = 10`.
   - S060 and `ExpiryRecoversWithinTtl`: pockets A then B; a consumer pulls A and
     stops; after the lapse a second consumer's pull claims A, not B, and receives A's
     records back at their original positions.
   - S062 and S024: three lapses in a row park A (count 3 with the default limit);
     the next pull claims B; a supersede, a stale publish or a new key into A requeues it
     at the back with count 0.
   - S064 and S056: one lapse (count 1); the consumer pulls A again, acks a record and
     renews; the lease lapses again and the count is 2, not 1; a release then resets it
     to 0, and three further lapses are needed to park.
   - S066 and S067, the branch-A pair, after PR 36's technique: publish, pull at 0 with
     `lease_ttl` 10 ms, clone the queue (a clone forks the state; the `FakeClock` handle
     is shared), set the clock to 11 once. On the first clone, tick then act: the tick
     lapses the lease, the pocket is queued at the front with its record requeued, and
     an action naming the old lease is `LeaseNotActive`. On the second clone, act then
     tick: the action succeeds, the tick lapses nothing, and for renew and ack
     `expires_at` is 21, the lease active and the pocket leased; for release the lease
     is released, absent from the registry, and the pocket queued at the back with a
     fresh position. Pull names no lease, so its pair differs: tick then pull claims the
     same pocket under a new lease, a different id, and the record is handed out again
     (`PullClaimsFrontPocket`); pull then tick is `PullWithinLease`, the batch empty
     because the record is still held (S037), `expires_at` 21, the lease active and the
     pocket leased, and the tick lapses nothing. S066 runs the pair for renew and for
     release (release first then tick is also S058's evidence, but S058 has its own
     test); S067 runs it for ack and for pull. Each order asserts the outcome, the lease
     and the pocket.
   - S069, after PR 36 line 386: pull, lapse, supersede the key, pull again under a new
     lease; ack, renew and release with the old lease are each `LeaseNotActive` and the
     snapshot is unchanged.
   - S048: supersede while held, nack (deferred), lapse: the deferred record is queued
     again at its deferred position and the next pull hands it out.
   - The backward step: clock at 100, pull; clock set back to 0; tick lapses nothing; a
     renew at clock 0 gives `expires_at` of 100 plus `lease_ttl`, not `lease_ttl`; a
     supersede at clock 0 stamps `shed_at` 100. Stamps never go backward and no lease
     lapses early.
   - The O(due) test, through the `testing` counts on `TickReport` and the counting
     wrapper in `tests/queue/counting.rs`: many leases active with three due, and a shed
     backlog of many entries none due; one tick reports three lapsed and zero decayed,
     `leases_examined` is four (the three due plus the later entry that stopped the
     walk, which the many active leases guarantee exists) and `shed_examined` is zero;
     separately, the wrapper counted exactly three `free_held` calls and no call on any
     other pocket. Shown to bite, on scratch copies never committed: `due` rewritten as
     a filter over the whole index makes `leases_examined` the active count, and the
     decay rewritten as a `retain` makes `shed_examined` the backlog size; the hand-back
     quotes both failures. The assertion is on counts, never on time.
   - S075: after a walk through publish, pull, ack, nack, release, lapse, park, revive and
     deferral, the snapshot's pocket positions are distinct and below `next_position`,
     each pocket's record positions likewise, and the queued order equals the sort by
     position.
   Run `just fmt-check`, `just clippy`, `just test`, `just doc`, `just coverage` and
   `just effects`.
5. **The tick lane.** Use the `rust-change` skill. In `actor.rs`, a second channel,
   `mpsc::channel::<Tick>(1)`, whose receiver the owner polls first:

   ```rust
   loop {
       tokio::select! {
           biased;
           tick = ticks.recv(), if ticks_open => match tick {
               Some(Tick { done }) => {
                   log(queue.tick());
                   let _ = done.send(());
               }
               None => ticks_open = false,
           },
           command = inbox.recv() => match command {
               Some(command) => apply(&mut queue, command),
               None => break,
           },
       }
   }
   ```

   `Tick { done: oneshot::Sender<()> }` carries a reply the owner sends once the tick is
   applied. The ticker task, started in `serve.rs` beside the owner and in the same
   `JoinSet`, loops: `tokio::time::sleep` for `--sweep-ms`, send one `Tick` on the lane
   and await its reply, then sleep again; so the first tick comes one sweep after start,
   the ticker never has a second tick in flight, and a dropped reply, the owner gone,
   ends the ticker. The owner loop ends when the command inbox closes, exactly as
   T24 left it; when the ticker is gone the tick branch yields `None` once, the arm
   clears `ticks_open`, and the `if` guard disables the branch from then on, so a closed
   lane never spins the `biased` select.
   Under a command backlog a tick waits for at most the command being applied, which is
   the bound 0019 states; ticks cannot starve commands because the lane holds one and a
   second tick is sent only after the first was applied and a full sweep has passed, so
   two ticks are separated by at least `--sweep-ms` of command service however long a
   tick's own application takes, and the owner spends at most one tick application per
   sweep. A unit test in `actor.rs` pins it with both lanes continuously ready: an owner
   over a small `#[cfg(test)]` inbox capacity, a test ticker running the protocol at
   the smallest positive sleep and never stopping, and a set of identified commands
   sent while the ticks keep arriving; each command's reply arrives within a bounded
   deadline that fails the test when it elapses, and the ticker is still being answered
   meanwhile, its count rising during the wait. No assertion orders a tick's
   acknowledgement against the commands sent before it: the `biased` select may answer
   a tick while an earlier command is still queued, so the test asserts progress, never
   order; the capacity and the sleep are the test's, small enough that the assertion is
   about the protocol, not the machine. Each
   tick that lapsed or decayed anything logs one `info` line with the three counts,
   never one line per lease at the default level.
   `--sweep-ms` in `config.rs`: `--name value`, default 1000, and 0, a negative or a
   non-numeric value is a usage error, as T24's flags are; `tests/binary.rs` pins the
   two refusals.
6. **The wire test.** In `crates/pachinko/tests/http.rs`, a server started with
   `--lease-ttl-ms 50 --sweep-ms 10`: join, publish, pull; then poll
   `GET /consumers/{consumer_id}/lease` until it answers 204, with a deadline of a few
   seconds that fails the test when reached (the deadline is the assertion; never assert
   an elapsed-time upper bound); then a second consumer's pull receives the record. Run
   `just test`.
7. **The handbook.** Use the `review-docs` skill. `docs/reference/http-api.md`: the lapse
   window paragraph says a lease can stay active past `expires_at` for up to one sweep
   plus the application of the tick in progress and of the command the owner is
   applying (a tick samples `now` once, so a lease expiring during a long tick waits
   for the next), that a pull, ack, nack or renew in that window succeeds and extends
   the lease, that a release in it succeeds by ending the lease and freeing its
   records, that neither is refused for being late (branch A, decision 0019), and that
   recovery after a dead consumer therefore takes `lease_ttl` plus that window;
   `docs/reference/configuration.md`, "Service flags":
   `--sweep-ms`; `docs/explanation/architecture.md`, "The service": the tick lane and
   why it is not the command FIFO; `CHANGELOG.md`: both Unreleased sections. Run
   `just check-docs`.
8. **Adversarial review** (`CONVENTIONS.md` §14): the whole diff against the Goal table,
   the scenario table, Files touched, the seven rules and the lapse section in
   `docs/specs/pachinko.allium`, and the loop in Step 5. Ask: whether a lapse and an
   action can interleave inside the owner, so that an owner borrow or a partial lapse
   crosses an `.await`; whether a lease can lapse twice, or lapse after a release;
   whether parking can skip a revive, or a revive leave the count above 0; whether the
   tick lane starves under a command backlog, commands starve under ticks, or the select
   spins when the ticker is gone; whether
   the O(due) test passes when the tick is O(n); whether the both-order tests pass with
   the lapse removed, or with the action's guard changed to read `expires_at`; whether
   the wire lapse test passes on its deadline; and whether any file outside Files
   touched changed. Codex `gpt-6-astra` reviewers run read-only with fresh contexts,
   one lens each; separate triage subagents write verdicts; valid fixes are applied one
   writer at a time; two rounds at most.
9. **Review, merge and gate.** Use the `review-change` skill over the diff, then the
   `project-check` skill: `just check`, with `fix-quality` for any failing recipe. Run
   `git fetch origin` (network, authorised by this step) and merge `origin/main`
   (`CONVENTIONS.md` §11): T27 or T29 may have merged since this branch was cut, so a
   conflict in a shared file keeps both tickets' lines and `Cargo.lock` is regenerated
   with `just lock` (network, authorised by this step), never merged by hand. Rerun the
   Verification block at the merge commit and requote it. Commit on the ticket branch
   with the hand-back notes, each quoted output headed by the commit it ran at. Pushing
   and opening the pull request are separately authorised: stop and ask.

## Acceptance criteria

- `Queue::tick` exists, returns `TickReport`, and the O(due) test shows a tick with
  three due leases among many active and a shed backlog none due examines four lease
  index entries and no shed entry, and touches exactly three pockets; the hand-back
  quotes the test failing under the full-scan `due` and the `retain` decay.
- A lease lapses when `expires_at <= now` and not before, once, and never after a
  release; after a lapse the old lease refuses every action.
- The three expiry rules select by the count and the record count in the order of
  Step 3; `ExpiriesWithinLimit` holds on every snapshot the tests take, and a parked
  pocket is requeued only by a publish.
- S066 and S067 each run both orders and assert the opposite outcomes: for renew, release
  and ack, refused after the lapse and succeeded before it; for pull, a new lease and the
  record redelivered after the lapse, the same lease extended and nothing redelivered
  before it.
- The 22 scenarios in the Goal table each have one library test tagged with their id,
  S026 is labelled OQ7, and the hand-back lists the obligation ids the tests carry.
- The owner polls the tick lane first in a `biased` select, the lane holds one tick, a
  `Tick` carries a reply the owner sends after applying it, the ticker awaits that reply
  before it sleeps the next `--sweep-ms`, the both-lanes-ready test passes, and the loop
  ends when the command inbox closes with the tick branch disabled once the ticker is
  gone.
- `--sweep-ms` defaults to 1000 and refuses 0 and negative values as usage errors.
- The wire lapse test observes 204 within its deadline and the second consumer's pull
  receives the record.
- `TickReport` is `Send + Sync + 'static`, `Clone`, `Debug` and `#[non_exhaustive]`, and
  every new public item has a doctest.
- `just coverage` reports at or above 90% of lines over `crates/*/src/**`, and
  `just effects` passes.
- `just check-docs` and `just check` are green at the commit that merges `origin/main`.
- The adversarial review in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just plan-spec pachinko
just fmt-check
just clippy
just test
just doc
just effects
just coverage
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

- Open question 7 stays open: S026 pins that only a publish revives a parked pocket, as
  the rules stand; an operator action or a cooldown would be a new rule.
- T19's empty-pocket-at-lapse reading is now code: below the threshold an empty pocket
  goes idle (S061) and at the threshold it parks (S063). The maintainer confirms, or
  chooses to keep an empty pocket queued at the front.
- A zero TTL is admitted and pinned (S006, S007) with no positive floor; whether any TTL
  needs one is the maintainer's.
- The monotonic clamp means a backward step of the system clock stretches every active
  lease by the step, and a host suspend stretches nothing until the clock is read again;
  a suspend-aware clock is the follow-up 0019 lists.
- The lapse-lag bound this ticket proves is qualitative; T30's probe measures it under
  load against the figures 0019 records.
- `TickReport` carries the lapsed leases in full so a caller can log them; if T29's
  `debug` level wants per-lease lines, it reads this report and adds nothing here.
