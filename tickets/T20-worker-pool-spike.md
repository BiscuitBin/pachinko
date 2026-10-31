---
id: T20
title: "Execution-model spike: bounded worker threads running single-threaded state machines over partitions"
status: done
depends_on: [T19]
parallel_with: []
branch: ticket/t20-worker-pool-spike
estimated_size: L
---

# T20: Execution-model spike: bounded worker threads running single-threaded state machines over partitions

## Context

T19 specified the domain in `docs/specs/pachinko.allium`: a queue of pockets, each
holding one record per key with a higher rank superseding in place, consumed one pocket
at a time under exclusive, renewable leases, with a lapse "handled in the same stream as
the consumer's actions". Nothing implements it yet, and the module excludes the thread
model, storage and transport on purpose. Decision 0014 defers the service's runtime until
a specification names a surface and says a spike then compares the options; T19's two
surfaces are that trigger. This ticket is the first half of that spike. Before any runtime
or transport is chosen it builds a prototype of the execution model the maintainer
proposed and measures it, so that the decision record which supersedes 0014, and the
implementation tickets that follow, rest on evidence rather than on a sketch. It takes no
decision: the maintainer takes it from the hand-back notes.

The proposal, in the maintainer's words on 2026-10-31: "a bounded number of long-lived
worker threads, each running single-threaded state machines for multiple partitions",
covering "the pocket queues as well as the lease manager". Four words carry the design,
and two of them collide with words `docs/project/terminology.md` already governs, so the
ticket fixes them here; the terminology page wins where this ticket's wording differs
from a domain word.

- **Worker thread.** One of a fixed number of threads started with the pool and joined
  when it stops, never spawned per request. Always "worker thread", never a bare
  "worker": the Consumer row forbids that word as a synonym for the consumer.
- **Partition.** A disjoint set of pockets, chosen by a stable function of the pocket
  key, whose state is touched by exactly one thread. A pocket is never a partition (the
  Pocket row forbids the word for a pocket); a partition holds many pockets.
- **Machine.** A single-threaded state machine: a value and a step function that takes
  one input and the current time as a value, changes the state and returns the outputs,
  with no clock read, thread, lock or randomness inside. The shape of the LMAX business
  logic processor, and the shape decision 0003 asks of library code.
- **The pocket queues**, the maintainer's phrase, are the pockets' records and the pocket
  order. **The lease manager** is the state that grants, extends, ends and expires leases
  and holds one active lease per consumer and per pocket; the module spreads it over
  `Lease`, `Pocket.status`, `Pocket.position`, `Pocket.consecutive_expiries` and
  `Queue.next_position`. Where that state lives is the spike's main question, and this
  ticket calls the answer the lease manager wherever it lands.
- **Pool.** The worker threads, the bounded queues between them, the clock that stamps
  each input, and the start and stop: everything the machines leave outside themselves.

### Sources

The maintainer named these. Each is read for the point in its row, and the hand-back
notes cite the row, not the page, when they lean on it.

| Source | What to take from it |
| --- | --- |
| `https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html` | A lease is a lock with a timeout, and its holder can keep acting after it has expired: a pause, a delayed message or a clock jump can put any action after `expires_at`, so safety cannot rest on timing. The resource checks a fencing token, a number that strictly increases with each grant, and rejects an action carrying an older one. The module already says each action names its lease (`ActionsNameTheirLease`): the lease identifier is the fencing token, and the machine that owns the pocket is the resource that checks it. |
| `https://martinfowler.com/articles/lmax.html` | One single-threaded business logic processor holds every piece of state in memory, is fed by an input disruptor and emits output events; six million orders a second on one thread; the state is a replay of the input journal, so a machine is tested and restarted by replay; the logic never calls out, it emits an event and resumes on a later input. The decisive finding for this ticket: an earlier prototype with actors and queues between them spent more time managing the queues than running the logic, which is why the one-machine configuration is this spike's baseline, not a straw man. |
| `https://lmax-exchange.github.io/disruptor/disruptor.html` | Why queues cost: head, tail and size take write contention and share cache lines; a bounded, pre-allocated ring buffer with a sequence counter per party removes the contention; a lagging consumer catches up in one batch; each field has one writer. The vocabulary for the spike's queue measurements, and the reason a queue between two machines is a cost to count, not free plumbing. |
| `https://github.com/LMAX-Exchange/disruptor` | The reference implementation, Java under Apache-2.0: ring buffer, sequencer, sequence barrier, wait strategy, event processor. Read for the shapes; the Rust port is the `disruptor` crate in the table below. |
| `https://technology.lmax.com/posts/concurrent-sequencing/` | Claiming sequence numbers from several threads with an atomic increment and an ordered publish went erratic under contention, the fix was a per-sequence state machine, and the post names this the one place the Disruptor breaks the single-writer principle. The module has one counter, `Queue.next_position`: it should have one writer, not be a shared atomic every partition increments. |
| `https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/` | A `go` statement is a `goto`: a task that outlives the block that started it breaks abstraction, cleanup and error propagation. A nursery keeps every task inside a block that does not exit until they all have, and an error in one cancels its siblings and surfaces in the parent. In Rust the nursery is `std::thread::scope`: the worker threads live inside one scope, stopping is a signal and then the scope's join, and a panic in one thread surfaces at the join rather than vanishing. |

### Crates surveyed

Checked on 2026-10-31 against the crates.io API (`https://crates.io/api/v1/crates/<name>`
and its `/versions` endpoint); every licence is in `deny.toml`'s allow list. Step 2
re-verifies each row on the day. On the pinned 1.99.1, `std::sync::mpmc` is still behind
the unstable `mpmc_channel` feature (error E0658, tracking issue
`https://github.com/rust-lang/rust/issues/126840`), while `std::sync::mpsc::sync_channel`
and `std::thread::scope` are stable; so the first cut needs no crate at all, and a crate
enters only when a measurement in step 6 shows what it would buy.

| Crate | Version | Licence | MSRV | What it would be for |
| --- | --- | --- | --- | --- |
| `crossbeam-channel` | 0.5.17 | MIT OR Apache-2.0 | 1.60 | Bounded multi-producer channels with `select!`; std's `mpsc` has been a port of it since Rust 1.67, so it buys multi-consumer receive and `select!`, not speed. |
| `crossbeam-utils` | 0.8.23 | MIT OR Apache-2.0 | 1.60 | `CachePadded`, against the false sharing the Disruptor paper describes, and `Backoff` for a spin-then-park wait. |
| `disruptor` | 4.4.0 | MIT | not stated | A Rust Disruptor: ring buffer, single and multi producer, batching, busy-spin wait strategies, thread pinning. The one crate that would change the architecture rather than the plumbing. |
| `rtrb` | 0.4.1 | MIT OR Apache-2.0 | 1.38 | A wait-free single-producer single-consumer ring buffer, for a fixed pair of machines. |
| `flume` | 0.12.0 | Apache-2.0/MIT | not stated | A multi-producer multi-consumer channel; an alternative to `crossbeam-channel`. |
| `parking_lot` | 0.13.0 | MIT OR Apache-2.0 | 1.95 | Smaller, faster locks. The design should need none inside a machine; a use of it is a finding. |
| `foldhash` | 0.2.0 | Zlib | 1.60 | Fast hashing for the pocket key to partition function. `std::hash::DefaultHasher::new()` is deterministic and sufficient; `RandomState` is randomness and would be banned in the library. |

### What the spike answers

Each question gets a section of the hand-back notes, in this order.

1. **The decomposition.** Which machine owns which state, what crosses between machines
   on each trigger the surfaces provide (`Publish`, `Pull`, `Ack`, `Nack`, `Renew`,
   `Release`) and on the lapse and the two decays, and how many hops each takes. The
   table is the hop table, and it is the hand-back's centrepiece.
2. **The counter.** Whether `Queue.next_position` must be one writer, and how pocket
   positions and record positions are assigned without a hop per publish.
3. **The lapse and the token.** How the clock enters each machine's stream as an input,
   so that a lapse and a consumer's action are ordered by one machine; and that an action
   naming an ended lease is rejected even when the same consumer holds the same pocket
   again, which is the fencing token doing its job.
4. **Ordering.** What order the pool preserves: a publisher's publishes into one pocket,
   a consumer's actions under one lease, and the clock's ticks against both; and what it
   does not preserve, said plainly.
5. **Bounded and structured.** That the pool never runs more threads than it was given,
   that each partition is stepped by one thread for the pool's whole life, that stopping
   joins every thread, that a panic in one surfaces to the caller, and that two machines
   sending to each other through full bounded queues cannot deadlock.
6. **One thread or many.** The measured throughput and latency of one machine on one
   thread against partitions over several worker threads, on one machine on one day, with
   the bottleneck named. LMAX's result says one thread may be enough; the numbers say
   whether it is here.
7. **The boundary.** What of the prototype would be libpachinko and what `pachinko` under
   decisions 0003 and 0008: the ticket's reading is that the machines are library code
   needing no effect trait at all, because a machine never spawns, sleeps or reads a
   clock, and the pool is service code that owns every effect. If any part of a machine
   turns out to need a thread or the clock otherwise than as an input, that is a finding
   and a decision record precedes it (`AGENTS.md`).
8. **Crates.** Which crates, if any, earned a place, with the before and after
   measurement for each; which were tried and dropped; and the decision-0007 note for
   each one kept.

### The ticket's reading, for the spike to confirm or refute

The module's chained triggers already draw the seams. `PocketTouched`, `Handed`, `Freed`
and `LeaseLapsed` are events one rule emits for another rule to answer, and in a
partitioned design they are candidate messages between machines. Two decompositions
follow from where the leases live:

- **A, leases central.** One order machine owns the pocket order, every lease and the
  expiry timers; the pocket machines own records. Every pull, ack, nack and renew extends
  `expires_at`, so each one reaches the order machine: a hop per record operation, and the
  order machine on the path of every consumer action.
- **B, leases with their pocket.** Each pocket machine owns its pockets' records, status
  and `consecutive_expiries`, and the leases on those pockets, and expires them from the
  ticks it receives. The order machine owns only the queued pocket order (position to
  pocket key and partition), the counter that positions pockets, and the consumer to
  active lease routing that holds `OneActiveLeasePerConsumer`. A publish into a queued
  pocket, an ack, a nack and a renew stay inside one pocket machine. The order machine
  hears pocket transitions only: a claim, a requeue, an idle, a park, a lease's end. A
  lease identifier that carries its partition routes an action with no table.

The ticket reads B's hop table as the smaller one and the spike builds B, recording A's
hop table from the rules alone. If building B shows the reading wrong, the spike builds
what the evidence says and the hand-back says why.

Under B a pull is two hops: the consumer's `Pull` reaches the order machine, which either
forwards it under the consumer's active lease or claims the front pocket, marks it leased
and forwards the claim; the pocket machine creates or checks the lease, selects the batch,
holds the records and answers with `Batch`. A release or a lapse is decided in the pocket
machine, which ends the lease, frees the records and tells the order machine what the
pocket became: requeued at the back, requeued keeping its position, idle or parked, and
that the consumer's lease ended. A publish that creates, revives or touches an idle or
parked pocket tells the order machine to enqueue it. Between a pocket machine's decision
and the order machine's hearing of it the pocket is in no state the module names; the
spike shows what can arrive in that window, a pull or a publish, and that the guards of
the two machines handle each.

Positions: pocket positions come from the order machine's counter, the single writer the
concurrent-sequencing post argues for. Record positions are assigned inside the pocket
machine from its own counter, combined with the partition number so that no two records
share one, which keeps `UniqueRecordPosition` as written; a record is only ever ordered
against records of its own pocket, and no surface exposes a position, so what a consumer
can observe is unchanged. This is a representation, not a rule change. Every place where
the prototype's representation departs from the module's literal model is listed in the
hand-back under "Representation", each with the argument that no surface can tell, and
handed to the maintainer as a question for a `spec-change` ticket; the spike resolves
none of them and touches no file under `docs/specs/`.

Time: the pool reads a monotonic clock once per input and stamps the input with it, and
injects a tick into every machine at a fixed interval so that an idle machine still
expires leases. Inside a machine `now` is a value from the input. Open question 8 of the
module, an action arriving after `expires_at` but before the lapse is processed, is
decided by the stream's order; the spike shows both orders occur and are each handled
consistently, and leaves the question open.

The two-full-queues deadlock is the risk this design carries: a worker thread blocked
sending to a full inbound queue of another worker thread that is itself blocked sending
back. The pool must make that impossible by construction, for example by never blocking a
worker thread on a send and holding the undelivered message in a local outbox that is
drained between inputs, and a test provokes the case with the smallest capacity.

### Claims this ticket makes, and the step that checks each

| # | Claim | Check | Outcome |
| --- | --- | --- | --- |
| 1 | A crate under `spikes/` with an empty `[workspace]` table is its own root, so every `--manifest-path` line reaches it and the root `Cargo.toml` is untouched; `tests/fixtures/effects-violation/Cargo.toml` is the precedent. | Step 3 | Holds. At 364d921 with uncommitted changes, committed as 952295a, all four gates reach `spikes/worker-pool/Cargo.toml` and the root `Cargo.toml` is untouched; see "Worktree and baseline". |
| 2 | `cargo deny --manifest-path spikes/worker-pool/Cargo.toml --config deny.toml --locked check licenses bans sources` applies the root policy to the spike's graph. | Step 3 | Holds. The same run prints `bans ok, licenses ok, sources ok` for the spike's graph, with the eleven license-not-encountered warnings an empty graph gives. |
| 3 | Renovate's cargo manager finds `spikes/worker-pool/Cargo.toml` by its default file pattern, so only the `rust-version` regex manager in `renovate.json` needs the path; the manager's documentation (`https://docs.renovatebot.com/modules/manager/cargo/`) is read on the day, because the dashboard shows it only after the merge. | Step 7 | Holds, per the Renovate page read on 2026-10-31; quoted under "Claims" in the hand-back notes. The dashboard check waits for the merge. |
| 4 | `std::sync::mpmc` is unstable on 1.99.1 and `std::sync::mpsc::sync_channel` and `std::thread::scope` are stable, so the first cut needs no dependency. Verified 2026-10-31 while writing this ticket, with a one-line probe. | Step 2 | Holds. E0658 on `std::sync::mpmc` and the `sync_channel` plus `thread::scope` probe compiles and runs; quoted under "Survey". No dependency was added. |
| 5 | The clippy pass over the spike inside `just check` costs an acceptable time; T16's precedent for `fuzz/` was 7.87 s cold and 0.06 s warm. | Step 9 | Holds. At 340e8d0 the spike adds 0.51 s to a cold `just clippy` (0.828 s total) and nothing measurable to a warm one (0.178 s total); quoted under "Claims" in the hand-back notes. |

Risks from `CONVENTIONS.md` §13 that apply: the toolchain pin, which moves the lint set
the spike is linted with; the network at first run and none in `check`; a gate that
cannot fire, which is why step 3 breaks each gate it adds a line to; and lychee and typos
over `tickets/`. Risks of this ticket's own: the deadlock above; a sweep on one laptop
read as a capacity claim, which is why every number names its machine and the hand-back
compares configurations rather than quoting absolutes; and a prototype mistaken for the
implementation, which is why the spike's `README.md` says what it is and the
implementation ticket starts from the module and `just plan-spec pachinko`, not from this
code.

## Goal

A crate under `spikes/worker-pool/`, outside the workspace with its own lockfile, that
runs every rule of `docs/specs/pachinko.allium` as single-threaded machines over
partitions on a bounded pool of long-lived worker threads under `std::thread::scope`;
tests that prove replay, the module's invariants, one thread per partition, the thread
bound, structured stop with panic propagation, the fencing token and freedom from the
two-full-queues deadlock; `just spike` printing a measured comparison of one machine on
one thread against partitions over several worker threads; `just spike-test` running the
spike's tests; the spike's manifest seen by the lock, format, clippy and deny gates;
hand-back notes that answer the eight questions above with the hop table, the
measurements, the boundary mapping and a recommendation; and `just check` green.

## Non-goals

- No decision. The record that supersedes 0014 is the maintainer's to take from the
  hand-back; the hand-back drafts its text under "Recommendation" and the maintainer
  decides. No file under `docs/decisions/` is created.
- No change to the module or to either crate: nothing under `docs/specs/`,
  `crates/libpachinko/` or `crates/pachinko/` changes, and no effect trait is written.
  The spike is a prototype the implementation ticket learns from, not its first commit.
- No transport, wire format or process boundary. Clients are threads in the same process.
- No persistence. The input log that proves replay lives in memory; journaling to disk is
  the implementation's question.
- No answer to any open question of the module, the eighth and the third included: the
  spike's back-pressure delays a publisher, it never refuses one, and whether to block or
  shed is open question 3's.
- No conformance claim. The machines run every rule, but `propagate` derives the
  implementation's tests in the implementation ticket; this ticket's tests prove the
  eight answers.
- No benchmark in CI, no change to `pyproject.toml`'s gate list, no new workflow and no
  `CHANGELOG.md` line, because neither crate changes.
- No optimisation of the machines themselves. The measurement is of the pool and its
  queues; a machine is written plainly.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `spikes/worker-pool/Cargo.toml` | new | `publish = false`, an empty `[workspace]` table, `rust-version = "1.99"`, the lint tables in the shape of `fuzz/Cargo.toml`, and each dependency with its decision-0007 note |
| `spikes/worker-pool/Cargo.lock` | new | Committed; the pin for the spike's separate graph |
| `spikes/worker-pool/README.md` | new | What the spike is and is not, how `just spike` and `just spike-test` run it, and that its findings are this ticket's hand-back notes |
| `spikes/worker-pool/src/` | new | The machines, the protocol between them, the partition function, the pool and the demo binary; the layout inside is the agent's, with unit tests in the module they test |
| `spikes/worker-pool/tests/` | new | The integration tests step 5 names |
| `Justfile` | shared, own lines | `spike` and `spike-test`; a `--manifest-path spikes/worker-pool/Cargo.toml` line in `sync`, `lock`, `lock-upgrade`, `lock-check`, `format`, `fix`, `fmt-check`, `clippy` and `deny`; the "three lockfiles" comment |
| `.pre-commit-config.yaml` | shared, own lines | The `fmt-check` hook's `files:` widened to `spikes/` |
| `.gitignore` | shared, own lines | `/spikes/*/target/` |
| `.dockerignore` | shared, own lines | `spikes/*/target/`, beside the fuzz lines |
| `renovate.json` | shared, own lines | The `rust-version` regex manager's file pattern, and the two descriptions that count cargo roots or name `fuzz/Cargo.toml` |
| `docs/project/repository-map.md` | shared, own lines | `spikes/` in the tree and one row in the table |
| `docs/reference/commands.md` | shared, own lines | The four lock recipes' rows, two Develop rows, the list of recipes outside `check`, the count of network recipes if it moves |
| `docs/reference/quality-gates.md` | shared, own lines | Gates 2, 4, 6 and 13 name the spike's manifest; the paragraph on `fuzz/` as its own workspace covers both |
| `docs/explanation/architecture.md` | shared, own lines | One paragraph under "What is deferred": the spike exists, what it showed, and that decision 0014 stands until its successor |
| `docs/project/purpose-and-scope.md` | shared, own lines | The runtime bullet names the spike as run and the decision as pending |
| `docs/tutorials/first-change.md` | shared, own lines | "syncs all three lockfiles": four. Step 7's grep finds any count this table missed |
| `docs/how-to/develop-locally.md` | shared, own lines | The lockfile count in the first-run paragraph, and the lockfiles `just sync` fetches. Step 7's grep finds any count this table missed |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | The two lockfile counts. Step 7's grep finds any count this table missed |
| `docs/how-to/test-and-debug.md` | shared, own lines | The lockfile count in the snapshot-accept step. Step 7's grep finds any count this table missed |
| `docs/operations/maintenance.md` | shared, own lines | The lockfile counts in the dependency routine, and the manifests that carry `rust-version`. Step 7's grep finds any count this table missed |
| `docs/reference/testing.md` | shared, own lines | The lockfile count in the snapshot section. Step 7's grep finds any count this table missed |
| `docs/decisions/0016-release-process.md` | shared, own lines | The lockfile count in the semver paragraph; the record's reasoning is untouched. Step 7's grep finds any count this table missed |
| `tickets/README.md` | shared, own lines | The index row's status, the opening paragraph, one dependency-graph bullet |
| `tickets/T20-worker-pool-spike.md` | ticket | `status:` and hand-back notes |

Not touched, and the adversarial reviews check it: `Cargo.toml`, `Cargo.lock`,
`deny.toml`, `pyproject.toml`, `docs/manifest.yml` (no new page), `CHANGELOG.md`,
everything under `crates/`, `docs/specs/`, `.github/`, and every decision record but
0016's count. Scratch, logs, probes and review files go under `ai_tmp/`.

## Steps

1. **Worktree and baseline.** Create the worktree from `main` after T19 has merged
   (`README.md`, "How to pick up a ticket"), run `just initialize` (network, authorised),
   then `just check` on the untouched worktree and quote its last line as the baseline.
2. **The survey, on the day.** Re-verify every row of the crate table (version, licence,
   MSRV, maintenance: last release, open issues, archived or not) and record the date.
   Re-run the `std::sync::mpmc` probe in `ai_tmp/` on the pinned toolchain and quote the
   error or its absence (claim 4). Decide, and write down, that the first cut uses std
   alone: `std::sync::mpsc::sync_channel` for the bounded inbound queues,
   `std::thread::scope` for the pool, `std::collections::BTreeMap` throughout (the root
   `clippy.toml` bans the hash containers, and clippy finds that file for the spike too),
   and `std::hash::DefaultHasher::new()` or an inline FNV-1a for the partition function.
3. **The crate and its gates, empty first.** Create `spikes/worker-pool/` with the manifest
   described in Files touched, a `lib.rs` holding the crate docs and a `main.rs` that
   prints usage, and generate `Cargo.lock` with
   `cargo generate-lockfile --manifest-path spikes/worker-pool/Cargo.toml` (network,
   authorised). Add the recipe lines, the hook pattern and the ignore lines. Add the two
   recipes:

   ```just
   # The execution-model spike (T20): the demo binary, release mode, arguments
   # through. Outside `just check`; its findings are the ticket's hand-back notes.
   spike *args:
       cargo run --manifest-path spikes/worker-pool/Cargo.toml --locked --release -- "$@"

   # The spike's unit, integration and doc tests. Outside `just check`, which lints the
   # crate but does not run it; a second spike generalises both recipes over a name.
   spike-test:
       cargo test --manifest-path spikes/worker-pool/Cargo.toml --locked
   ```

   Run `just lock-check`, `just fmt-check`, `just clippy` and `just deny` on the empty
   crate and quote them (claims 1 and 2). Then break each once, in the worktree and
   reverted before the next, and quote the failure: an unformatted line for `fmt-check`,
   a `let x = 1;` nobody reads for `clippy`, a version in `Cargo.lock` edited by hand for
   `lock-check`. A gate that cannot be shown to fire on the spike is not trusted to hold
   it (`CONVENTIONS.md` §13).
4. **The machines.** Use the `rust-change` skill for the shape of the code even though
   the crate is outside `crates/`: the lint tables are the workspace's. Write the order
   machine and the pocket machine of decomposition B, or of what the evidence says, and
   the protocol between them as one enum of inputs and one of outputs, with the lease
   identifier carrying its partition and acting as the fencing token. Every rule of the
   module is a branch of a step function, in the simplest form that is faithful: the four
   publish rules and the two revives, the three pull rules and `HoldRecord`, the two acks,
   the two nacks, `Renew`, the two releases and the two requeues, `LeaseLapses` and the
   three expiry rules, the two decays. A machine takes `now` as a value and never reads a
   clock, spawns, sleeps or locks; a unit test in each module proves a rule against the
   model written from its `requires` clauses (`docs/reference/testing.md`, "Tests from
   the specifications"). Write the two single-threaded integration tests here, before
   there is a pool: one replays each machine's recorded input log into a fresh machine and
   asserts the states equal; one runs a scripted sequence through the machines directly,
   accepted and refused actions both, and after every step asserts each invariant of the
   module by name as far as the aggregate state shows it. Record the hop table and the
   "Representation" list as you go; both are easier to write while the code is being
   written than after.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint 1, over the machines before
   the pool builds on them. Ask: whether each rule of the module is a branch, with its
   guards, and none is a reading the module does not state; whether every departure from
   the literal model is in the "Representation" list with an unobservability argument
   that holds, and whether any is in fact observable through `Consuming`'s `exposes`;
   whether an action naming an ended lease can reach a record under a successor lease;
   whether the window between a pocket machine's decision and the order machine's hearing
   of it can lose a pocket, double-claim one or leave one in a state no rule leaves;
   whether the invariant test can pass while an invariant is broken (it asserts after
   every step, not only at the end, and its oracle is the `requires` clauses, not the
   machine); and whether `now` enters a machine any other way than as an input.
5. **The pool.** Under `std::thread::scope`, start the number of worker threads the caller
   gives, assign the partitions to them statically, give each worker thread one bounded
   `sync_channel`, stamp each input with the monotonic clock as it is dequeued, inject a
   tick into every machine on a fixed interval, route a pocket key to its partition and a
   lease to its partition and a bare consumer action to the order machine, never block a
   worker thread on a send (the outbox rule, or whatever construction the agent proves),
   and stop by a signal followed by the scope's join. Write the integration tests, each
   named after the clause it proves; these names are suggestions:
   `each_partition_is_stepped_by_one_thread_and_never_more_than_the_pool_has` (machines
   record the stepping thread's id; the set per partition has one member and the set over
   all has at most the pool's size),
   `stopping_joins_every_worker_thread_and_a_panic_reaches_the_caller` (a machine told to
   panic on a test input; the scope's join returns the panic; no thread outlives it),
   `an_action_naming_an_ended_lease_is_rejected_under_a_successor_lease` (expire a lease
   by advancing the clock, let the same consumer claim the same pocket again, replay the
   old lease's ack and see it refused while the records stay as they were),
   `two_machines_flooding_each_other_through_full_queues_make_progress` (capacity 1,
   two partitions on two worker threads publishing into each other's pockets as fast as
   they can, bounded wall time),
   `a_publisher_s_publishes_into_one_pocket_arrive_in_order` and
   `a_lapse_and_a_late_renewal_are_ordered_by_one_machine_either_way` (both orders
   produced, each handled as the rules say for that order). Then, for the invariant,
   fencing and single-thread tests, break the code on purpose, quote the failing test's
   name, and restore it (`docs/reference/testing.md`, "A test that passed when first run
   is shown to bite").
6. **The measurement.** The demo binary takes the worker-thread count, the partition
   count, the publisher and consumer counts, the pocket count, `lease_ttl` and a
   duration, runs a synthetic load (publishers publishing into pockets with repeated keys
   so that supersedes happen; consumers pulling, acking most, nacking some, releasing, and
   one consumer stalling past `lease_ttl` so that lapses happen), and prints for the run:
   operations a second by class, median and 99th percentile latency by class, the count
   of messages that crossed between machines, and the invariants checked at the end.
   With no arguments it sweeps at least (1 worker thread, 1 partition), (1, P) and (N, P)
   with N the host's `available_parallelism` and P at least 16 and at least N, for a few
   seconds each, and prints one table. Quote the table in the hand-back with the host's
   operating system and core count; the numbers compare configurations on one host on one
   day and claim nothing else. Only if the table names the channels as the bottleneck,
   put `crossbeam-channel` or `disruptor` behind the same interface, re-measure, and keep
   it with its decision-0007 note or drop it; quote both tables either way. The
   `multiple_crate_versions` lint stays `allow` as in `fuzz/Cargo.toml`; cargo-deny's
   duplicate check covers it.
7. **The handbook ripple.** Use the `review-docs` skill. The pages in Files touched; then
   `grep -rn -i 'three lock\|all three\|fuzz/Cargo.lock' docs/ Justfile renovate.json`
   and fix every count or list this ticket's table missed; then `grep -rn -i spike docs/`
   and make every page that describes the spike as future say it ran as T20 and what it
   found, without taking the decision. Read Renovate's cargo manager page for claim 3.
   `tickets/README.md`: the index row and the opening paragraph, and one bullet under the
   dependency graph. Run `just check-docs`.
8. **The hand-back notes.** Write them in the order of "What the spike answers", with
   these sections, each quote headed by its commit: Worktree and baseline; Survey; The
   decomposition and the hop table; The counter and positions; The lapse and the token;
   Ordering; Bounded and structured; Measurement; The boundary; Crates; Representation;
   Recommendation, which drafts the text of the record that would supersede 0014 and the
   outline of the implementation ticket, both as proposals; Claims, with the table's
   Outcome column filled; and the adversarial reviews.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint 2, over the whole diff before
   the final commit. Ask: whether anything under `crates/`, `docs/specs/`,
   `docs/decisions/` other than 0016's count, or `Cargo.lock` changed; whether every
   quoted number has a commit and a host; whether the hop table covers every trigger in
   both surfaces plus the lapse and the decays; whether the Recommendation says anything
   the measurements do not support; whether a page now says the decision is taken;
   whether the four gates were shown to fire on the spike; whether `just check` still
   needs no network; and whether the terminology collisions in Context leaked into the
   handbook as bare "worker" or "partition" for a pocket.
9. **The gate.** `just check`. Then once, after `just sync`, `just check` under the
   network denial `CONVENTIONS.md` §11 describes, with its two negative controls quoted
   inside the same denial; a sandbox that fails to launch is recorded, not worked around.
   Time the `clippy` recipe cold and warm for claim 5. Commit on the ticket branch with
   `status: done` and the hand-back notes. **Authorisation required**: `git fetch origin`,
   the rebase or merge, the push and the pull request are each separately authorised; stop
   and ask.

## Acceptance criteria

- `just spike-test` exits 0, and its output names the replay, invariant, one-thread,
  stop-and-panic, fencing, deadlock, publish-order and lapse-order tests.
- For the invariant, fencing and one-thread tests, a deliberate break is quoted with the
  failing test's name.
- `just spike` with no arguments prints the sweep table for at least (1, 1), (1, P) and
  (N, P), and the hand-back quotes it with the host named.
- The hand-back's hop table has a row for `Publish` into a new, a queued, an idle and a
  parked pocket, for `Pull` with and without an active lease, for `Ack`, `Nack`, `Renew`
  and `Release`, for the lapse, and for the two decays, each naming the machines touched
  and the messages that crossed.
- The "Representation" list names every departure from the module's literal model, each
  with its unobservability argument, and no file under `docs/specs/` changed.
- `git diff --stat main -- crates docs/specs Cargo.toml Cargo.lock deny.toml` prints
  nothing.
- `just lock-check`, `just fmt-check`, `just clippy` and `just deny` reach the spike's
  manifest, and the first three were each shown to fail on a deliberate break in it.
- Every dependency the spike keeps carries a decision-0007 note beside its range.
- The handbook counts four lockfiles wherever it counted three, and no page says the
  decision that supersedes 0014 is taken.
- `just check` is green, and once under network denial with both negative controls
  failing, or the sandbox failure is recorded.
- The two adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid
  finding is fixed or handed back.

## Verification

```sh
just spike-test
just spike
just lock-check && just fmt-check && just clippy && just deny
just check-docs
just check
git diff --stat main -- crates docs/specs Cargo.toml Cargo.lock deny.toml
git status --short
```

## Hand-back notes

Written on 2026-10-31. Every number below was measured on one host, "the host" from
here on: macOS 26.4, Apple M5 Pro, 18 cores (6 performance and 12 efficiency), 64 GiB.
The numbers compare configurations on that host on that day and claim nothing else; no
capacity claim is made anywhere in these notes. Every quoted output is headed by the
commit it ran at, with "with uncommitted changes" where that was true, and shows its exit
status where the recording file holds one; where a file holds no exit status the notes
say so. The final code commit is 340e8d0, which carries the checkpoint 4 and code-review
fixes on top of ce593ec's checkpoint 3 round 2 fixes; the quotable sweep, the client
scaling, the clippy timing and the gate were taken at it, on a clean worktree.

### Worktree and baseline

The worktree is on branch `tickets/t20-worker-pool-spike-complete`, not the ticket's
`ticket/t20-worker-pool-spike`: it was prepared under that name and kept as prepared, as
T16's merges were (Open points). `just initialize` was not run: `.pixi/` and `.tools/bin`
(allium, cargo-fuzz, cargo-hack, cargo-qual, cargo-release, rustqual) were present from
the worktree's preparation, so no network recipe ran for the baseline.

Baseline on the untouched worktree, at 364d921, last lines of `just check`
(`ai_tmp/t20-baseline-check.log`, 24.6 s wall):

```text
All checks passed and the worktree is unchanged.
just check  55.75s user 14.02s system 283% cpu 24.616 total
exit=0
```

Step 3 built the empty crate and wired its gates. The gate runs were recorded at 364d921
with uncommitted changes, which became commit 952295a. Green, decisive lines
(`ai_tmp/gates/green-empty.log`; the cargo-deny warning boxes and colour codes are
omitted):

```text
commit 364d921 with uncommitted changes
$ just lock-check
cargo update --workspace --locked --offline --manifest-path spikes/worker-pool/Cargo.toml
     Locking 0 packages to highest Rust 1.99 compatible versions
exit=0
$ just fmt-check
cargo fmt --manifest-path spikes/worker-pool/Cargo.toml --check
exit=0
$ just clippy
cargo clippy --manifest-path spikes/worker-pool/Cargo.toml --all-targets --locked -- -D warnings
    Checking worker-pool v0.0.0 (.../spikes/worker-pool)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
exit=0
$ just deny
cargo deny --manifest-path spikes/worker-pool/Cargo.toml --config deny.toml --locked check licenses bans sources
bans ok, licenses ok, sources ok
exit=0
```

Each of the three gates was then broken once in the worktree and restored before the
next (`ai_tmp/gates/breaks-empty.log`, same commit heading; colour codes stripped):

```text
### break 1: fmt-check, an unformatted line in src/main.rs
Diff in .../spikes/worker-pool/src/main.rs:13:
-fn   unformatted( ) { }
+fn unformatted() {}
error: recipe `fmt-check` failed on line 218 with exit code 1
exit=1

### break 2: clippy, a `let x = 1;` nobody reads in src/main.rs
error: unused variable: `x`
  --> src/main.rs:14:9
   = note: `-D unused-variables` implied by `-D warnings`
error: could not compile `worker-pool` (bin "worker-pool") due to 1 previous error
error: recipe `clippy` failed on line 229 with exit code 101
exit=101

### break 3: lock-check, the version in Cargo.lock edited by hand
-version = "0.0.0"
+version = "0.0.1"
 Downgrading worker-pool v0.0.1 (.../spikes/worker-pool) -> v0.0.0
error: cannot update the lock file .../spikes/worker-pool/Cargo.lock because --locked was passed to prevent this
error: recipe `lock-check` failed on line 90 with exit code 101
exit=101

### restored
$ just lock-check && just fmt-check
exit=0
```

The spike's lockfile holds one package and was generated with
`cargo generate-lockfile --offline`: with no dependency, step 3 needed no network after
all. `git diff --stat main -- crates docs/specs Cargo.toml Cargo.lock deny.toml` at
3de4bc1 with uncommitted changes prints nothing (exit 0), and
`git diff --stat main -- docs/decisions` lists only
`docs/decisions/0016-release-process.md | 2 +-`, the lockfile count; checkpoint 4's
reviewer ran the same commands at 913f1b1, exit 0, and found nothing more.

### Gate

`just check` on the clean worktree at 340e8d0 (`ai_tmp/t20-check-340e8d0.log`, 12:53:36
PDT). Every recipe from `check-toolchain` to `check-clean` ran; the first and last lines:

```text
commit 340e8d0, 0 uncommitted paths
$ just check
bg-project-check run

==> just check-toolchain
...
==> just check-clean
bg-project-check clean "$1"
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit=0
```

Then once more with the finished ticket as the only uncommitted path, so the quote covers the
hand-back notes themselves (`ai_tmp/t20-check-final.log`); 340e8d0 with uncommitted changes
(`tickets/T20-worker-pool-spike.md`), exit 0:

```text
$ just check
...
All checks passed and the worktree is unchanged.
exit=0
```

Then once under the network denial `CONVENTIONS.md` §11 describes, at 340e8d0
(`ai_tmp/t20-check-sandbox-340e8d0.log`, 12:53:45 PDT): the two negative controls, then
the gate, as three invocations of the same profile, the form T10 and T11 used. Step 9
says "after `just sync`"; `just sync` was not run first, because it reaches the network
and the ordinary run above had just shown the environment current (`just lock-check`
ran `cargo update --workspace --locked --offline` on all three manifests and
`pixi lock --check --offline --dry-run`, each already up to date). No sandbox failed to
launch.

```text
commit 340e8d0, 0 uncommitted paths
$ sandbox-exec -p '(version 1)(allow default)(deny network*)' curl -fsS --max-time 10 https://static.rust-lang.org
curl: (6) Could not resolve host: static.rust-lang.org
exit=6

$ sandbox-exec -p '(version 1)(allow default)(deny network*)' curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1
curl: (7) Failed to connect to 1.1.1.1 port 443 after 0 ms: Couldn't connect to server
exit=7

$ sandbox-exec -p '(version 1)(allow default)(deny network*)' just check
bg-project-check run

==> just check-toolchain
...
==> just check-clean
bg-project-check clean "$1"
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit=0
```

### Survey

Re-verified on 2026-10-31 against the crates.io API (fourteen reads) and the GitHub
repository API (six reads), every response HTTP 200, no token sent
(`ai_tmp/t20-survey.md`). All seven rows are unchanged on version, licence and MSRV; no
version is yanked; no repository is archived; every licence is in `deny.toml`'s allow
list, flume's legacy `Apache-2.0/MIT` read as an OR. Last pushes: crossbeam 2026-10-01,
disruptor 2026-08-08, rtrb 2026-10-30, flume 2026-10-15, parking_lot 2026-10-21,
foldhash 2026-07-03. One note: crates.io reports rtrb's `newest_version` as 0.3.6 because
a 0.3.x backport was published seven minutes after 0.4.1; the table uses 0.4.1, the
highest non-yanked version, which matches the ticket row.

The `std::sync::mpmc` probe, rerun while writing these notes at 3de4bc1 with uncommitted
changes, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`:

```text
$ rustc --edition 2024 -o ai_tmp/probe/mpmc ai_tmp/probe/mpmc.rs
error[E0658]: use of unstable library feature `mpmc_channel`
 --> ai_tmp/probe/mpmc.rs:2:20
  |
2 |     let (tx, rx) = std::sync::mpmc::channel::<u8>();
  |                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: see issue #126840 <https://github.com/rust-lang/rust/issues/126840> for more information
error: aborting due to 3 previous errors
exit 1
```

The companion probe `ai_tmp/probe/stable.rs` (`sync_channel` plus `thread::scope`)
compiled and ran during step 2; its binary is beside it. Claim 4 holds.

Decided and written down in step 2: the first cut uses std alone. `sync_channel` for
the bounded inbound lanes, `std::thread::scope` for the pool, `BTreeMap` and `BTreeSet`
throughout (the root `clippy.toml` bans the hash containers and reaches the spike), and
an inline 64-bit FNV-1a in `src/partition.rs` for the pocket key to partition function,
tested against the reference vectors. No crate entered later either (Crates).

### The decomposition and the hop table

B was built, leases with their pocket, and the evidence did not turn the reading over.
A hop is one bounded-queue traversal on the path from the client to its reply, the
client's own entry counted; "messages crossed" counts every message between machines,
the notifications after the reply included. The rule names are the module's.

| Trigger | Machines touched | Messages crossed | Hops to reply |
| --- | --- | --- | --- |
| `PublisherJoins`, `ConsumerJoins` | order | none | 1 |
| Publish, new pocket (`PublishIntoNewPocket`) | pocket, order | `Enqueue` | 1 |
| Publish, queued or leased pocket (`PublishNewKey`, `Supersede`, `DropStalePublish`) | pocket | none | 1 |
| Publish, idle pocket (`ReviveIdlePocket`) | pocket, order | `Enqueue` | 1 |
| Publish, parked pocket (`ReviveParkedPocket`) | pocket, order | `Enqueue` | 1 |
| Pull, no active lease, a pocket queued (`PullClaimsFrontPocket` and `HoldRecord`) | order, pocket | `Claim` | 2 |
| Pull, no active lease, none queued (`PullFindsNoPocket`) | order | none | 1 |
| Pull, active lease (`PullWithinLease`) | order, pocket | `PullUnderLease`; in the window, `RetryPull` back to the order machine and a fresh `Claim` | 2; in the window, 4 when the retry claims a pocket (the `Claim` is a hop) or 3 when it finds none queued |
| Ack (`AckSettlesRecord`, `AckDefersNewerVersion`) | pocket | none | 1 |
| Nack (`NackSettlesRecord`, `NackDefersNewerVersion`) | pocket | none | 1 |
| Renew (`Renew`) | pocket | none | 1 |
| Release (`ReleaseRequeuesPocket` with the two requeues, `ReleaseIdlesEmptyPocket`) | pocket, order | `PocketBecame` | 1 |
| Lapse (`LeaseLapses`, then `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront` or `LeaseExpiryIdlesEmptyPocket`, with the two requeues) | pocket, order | `PocketBecame` | none: the tick is the input |
| `SupersededRecordDecays` (tick) | pocket | none | none |
| `DeadLetterDecays` (tick) | pocket | none | none |

A, leases central, from the rules alone, since nothing of A was built: the publish rows
as B; a claiming pull is order to pocket for the batch and back for the lease, 2 hops; a
pull within a lease is order (extend `expires_at`) then pocket (batch), 2; ack and nack
are order (the lease guards, extend `expires_at`) and pocket (the record), 2 hops or 1
plus a message; renew is order alone, since the rule touches only the lease, 1 hop;
release is order (end the lease) then pocket (free the records), 2; the lapse is
detected in the order machine and frees records in the pocket machine, 1 message. The
joins and the no-pocket pull are order alone, 1 hop, as under B; the two decays are
pocket on the tick, no hop, as under B. Under A every consumer action reaches the order
machine; under B only pulls and lease ends do.

The window the Context names, between a pocket machine's decision and the order
machine's hearing of it, was shown and guarded. A `Claim` always finds its pocket queued:
a publish into a queued pocket emits no `Enqueue`, so a publish overtaking a `Claim`
changes nothing the claim reads. A `RetryPull` is handled after the `PocketBecame` that
ended the lease: the pocket machine answers a `PullUnderLease` under an ended lease with
`RetryPull` to the order machine, and because everything one machine sends another
travels through one outbox into one inbound lane, that retry cannot overtake the
`PocketBecame` sent before it. `tests/window.rs` steps the order machine and one pocket
machine by hand through four windows (a second pull before its `Claim` is delivered; a
pull racing a lapse; a release followed at once by a pull; a publish overtaking a
`Claim`), each ending at quiescence with zero violations, every record handed, one active
lease and the order view consistent. The fifth hand-stepped probe
(`ai_tmp/probe-1/tests/window.rs`, checkpoint 1) shows what the FIFO rule protects:
with `RetryPull` delivered before `PocketBecame` the order machine routes the retry
under the ended lease again and the pair loops. The flood test exercises the window
under load with capacity 1: in one `--nocapture` run after the two-lane split, before
the pool's commit 110e006 and from the pool agent's report (`ai_tmp/t20-agent-pool.md`),
worker thread 0 stepped 94,504 inputs and crossed 35,157 messages (31,558 local, 3,599
over the channel, outbox high water 13) and worker thread 1 stepped 32,198 and crossed
30,365 (high water 272), in 307 ms, with every invariant holding at the end.

### The counter and positions

`Queue.next_position` need not be one writer, but each of its two halves must be, and
the spike gives it two single writers: the order machine's `next_pocket_position` for
pockets and each pocket machine's `next_record_serial` for records, a record's position
being `serial * partitions + partition` so that no two records anywhere share one. A
publish therefore takes its record position inside the pocket machine with no hop; a
pocket position is taken by the order machine when it hears the `Enqueue`, which is a
message the publish sends anyway, after its reply. The concurrent-sequencing row's
argument holds for each counter: one writer, no shared atomic. This is Representation
item 1, and `UniqueRecordPosition`, `UniquePocketPosition` and the two `BelowNext`
invariants are checked over the split counters at every step of the scripted test.

### The lapse and the token

The clock enters a machine only as `Input::Tick` and as the `now` stamped on every
input: the pocket machine expires leases from the ticks it receives, in the same stream
as the consumer's actions, so a lapse and an action are ordered by one machine. The
lease identifier is `LeaseId { partition, serial }`, minted by the order machine with a
strictly increasing serial, kept by the pocket machine after the lease ends; it routes
the action to its pocket machine with no table and is the fencing token. The guard
shared by ack, nack, renew and release checks `UnknownLease`, then `LeaseNotActive`,
then `NotYourLease`, before any record is read, so an action naming an ended lease is
`Rejected(LeaseNotActive)` and never reaches a record under a successor lease, even when
the same consumer holds the same pocket again. Expiry fires only on a tick and includes
equality (`expires_at <= now`).

The fencing case is tested in three places: the pocket machine's unit test
`ack_naming_an_ended_lease_is_rejected_despite_a_successor_lease`
(`src/pocket/tests.rs`), the single-threaded
`an_action_naming_an_ended_lease_is_rejected_under_the_successor_lease_and_records_are_unchanged`
(`tests/invariants.rs`) and the pool's
`an_action_naming_an_ended_lease_is_rejected_under_a_successor_lease` (`tests/pool.rs`),
which expires the first lease by the clock, lets the same consumer claim the same pocket
again, replays the old lease's ack and asserts the refusal, the record still held with
its rank, held rank, position, payload and held payload unchanged, the first lease
`Expired` and the successor `Active`. The both-orders test runs twice:
`a_lapse_and_a_late_renewal_are_ordered_by_the_pocket_machine_either_way` through
`Direct`, and `a_lapse_and_a_late_renewal_are_ordered_by_one_machine_either_way` through
the pool, whose lapse half (TTL 30 ms, 5 ms tick, 100 ms sleep) sees the renewal
`Rejected(LeaseNotActive)` and whose renewal half (TTL 30 ms, 2 s tick, 100 ms sleep)
sees a renewal stamped at least 70 ms after the lease expired by the clock accepted,
with zero ticks run, because no lapse had been processed. Both orders occur and each is
handled as the rules say for that order; open question 8 stays open.

### Ordering

Preserved: a publisher's publishes into one pocket (one lane, one machine, FIFO); a
consumer's actions under one lease (one pocket machine; a `Pull` goes through the order
machine first, and a consumer acts after each reply); the clock's ticks against both
within one machine (one stream). Not preserved, said plainly: a `Published` reply can
arrive before the pocket is pullable, its `Enqueue` still in flight; pockets created in
different partitions enter the pocket order in the order the order machine hears their
`Enqueue`s, not in publish order, even from one publisher; a `Pull` may answer
`NoPocketAvailable` while a just-released pocket's `PocketBecame` is in flight; a `Pull`
processed before a lapse's `PocketBecame { RequeuedKeepingPosition }` claims the next
queued pocket rather than the one the module puts at the front (checkpoint 1, C1); two
consumers' actions on different pockets are unordered.

`Direct`, the single-threaded router the replay and invariant tests drive, cannot
interleave a client input with the messages chained from an earlier one: `submit` and
`tick` each run the machines to quiescence before returning, so no test through it
reaches the window and its retry counter is always 0. `tests/window.rs` covers the
windows by hand (above), and the pool exhibits them under load.

The load made the Release-then-Pull window visible. Before the checkpoint 2 fix B8,
at 110e006, the order machine's retry counter was 43,154 over a 4.26 s pool life on one
worker thread, about a third of all pulls, and 72,233 with six worker threads, about
half: the consumer's `Released` reply reached it before the order machine had stepped
the `PocketBecame`, so the next `Pull` went out under the ended lease, came back as
`RetryPull`, and only then claimed, two extra hops. After B8 (a client-lane burst ends
as soon as a step has queued a message for one of the worker thread's own machines) the
counter is 0 on one worker thread in every run since, and roughly halved across worker
threads: at 340e8d0, 25,837 to 27,573 with six and 19,706 to 19,810 with eighteen. What
remains is the cross-worker-thread case, a `PocketBecame` on the peer lane overtaken by
the next `Pull` on the client lane, which the protocol tolerates.

### Bounded and structured

The pool as built: the caller's number of worker threads, started under
`std::thread::scope` and joined when it stops; partition `p` on worker thread `p mod N`
and the order machine on worker thread 0, statically; two bounded `sync_channel` lanes
per worker thread, a client lane that clients block on and a peer lane that worker
threads `try_send` into; a message whose lane is full waits in the sender's outbox,
drained between inputs, so no worker thread ever blocks on a send; the monotonic clock
read once per input as it is dequeued; a tick into every machine on a fixed interval;
`park_timeout` and `unpark` for idling, since std has no select over two receivers;
stop is a signal and then the scope's join, and a panic in a worker thread or the body
surfaces at the join.

The six integration tests, with their wall times run alone from the pool agent's report
(`ai_tmp/t20-agent-pool.md`, which records no commit and no exit status; the tests are
the ones first committed in 110e006, measured on the pool files before they landed):

| Test | Time |
| --- | --- |
| `each_partition_is_stepped_by_one_thread_and_never_more_than_the_pool_has` | 0.20 s |
| `stopping_joins_every_worker_thread_and_a_panic_reaches_the_caller` | 0.00 s |
| `an_action_naming_an_ended_lease_is_rejected_under_a_successor_lease` | 0.26 s |
| `two_machines_flooding_each_other_through_full_queues_make_progress` | 0.37 s |
| `a_publisher_s_publishes_into_one_pocket_arrive_in_order` | 0.00 s |
| `a_lapse_and_a_late_renewal_are_ordered_by_one_machine_either_way` | 0.14 s |

Every test ends with the invariant check empty and every machine's violation counter 0;
the stop test's panic half asserts the payload, that `thread::scope` returned, and that
no worker thread is live. `just spike-test` at 3de4bc1 with uncommitted changes (the
checkpoint 3 round 2 fixes, landing in ce593ec;
`ai_tmp/review/T20-3b/spike-test.log`, exit 0 per the apply report beside it) names
every test the acceptance criteria ask for; at 913f1b1 with uncommitted changes (the
checkpoint 4 and code-review fixes, landing in 340e8d0;
`ai_tmp/review/T20-4/apply-spike-test.log`, exit 0 per `apply-code-report.md`) the same
targets passed with the six new unit tests: 161, 6, 5, 6, 1 and 4 unit and integration
tests and 22 doctests, 0 failed:

```text
     Running tests/invariants.rs
test consecutive_pulls_under_one_lease_yield_exactly_one_reply_each ... ok
test an_action_naming_an_ended_lease_is_rejected_under_the_successor_lease_and_records_are_unchanged ... ok
test one_pocket_became_per_lease_end_over_all_four_paths ... ok
test a_lapse_and_a_late_renewal_are_ordered_by_the_pocket_machine_either_way ... ok
test every_invariant_holds_after_every_step_of_a_scripted_sequence_with_refusals ... ok
     Running tests/pool.rs
test stopping_joins_every_worker_thread_and_a_panic_reaches_the_caller ... ok
test a_publisher_s_publishes_into_one_pocket_arrive_in_order ... ok
test each_partition_is_stepped_by_one_thread_and_never_more_than_the_pool_has ... ok
test a_lapse_and_a_late_renewal_are_ordered_by_one_machine_either_way ... ok
test an_action_naming_an_ended_lease_is_rejected_under_a_successor_lease ... ok
test two_machines_flooding_each_other_through_full_queues_make_progress ... ok
     Running tests/replay.rs
test each_machine_replayed_from_its_input_log_equals_itself ... ok
     Running tests/window.rs
test a_publish_overtaking_a_claim_leaves_the_pocket_claimable ... ok
test a_second_pull_before_the_first_claim_is_processed_is_forwarded_behind_it ... ok
test a_release_followed_at_once_by_a_pull_is_retried_and_claims_the_pocket_at_the_back ... ok
test a_pull_racing_a_lapse_is_retried_after_the_pocket_became_is_heard ... ok
```

**The two-lane finding.** The first cut gave each worker thread one inbound channel, and
the flood test starved machine-to-machine traffic: a client blocked in `send` on a full
`sync_channel` wins the freed slot from a peer's `try_send` nearly every time. Measured
on the first cut of the flood test (capacity 1, two worker threads, four publishers and
four consumers, 300 ms; from the pool agent's report, before 110e006): worker thread 0
delivered 49 messages to worker thread 1 in the whole run, worker thread 1's outbox
reached 22,150 waiting `Enqueue`s, a consumer's `Pull` got no reply in 5 s, and the test
took 27.06 s to drain the backlog at stop. After the split into a client lane and a peer
lane: 3,599 deliveries from worker thread 0 to 1, high water 272, 307 ms. FIFO per
destination survives the split, because everything one machine sends another travels
through the same outbox into the same peer lane.

**Deadlock freedom.** No worker thread ever blocks on a send: a message whose
destination lane is full waits in the sender's outbox, and the sender goes on to drain
its own lanes every iteration whatever its peers are doing. So every full lane is always
being emptied by a worker thread that is never itself waiting, every pending `try_send`
eventually succeeds, and the one blocking send in the pool, a client's, blocks on a lane
a live worker thread drains. The flood test provokes the case at capacity 1 and asserts
that both peer lanes did fill (a `lane_full` counter on both worker threads, added at
checkpoint 2), that messages crossed both ways, one reply per request, and a 10 s bound
enforced by a watchdog thread.

**The fairness bound.** Two things stay unfair. Client against client on the client lane
is unordered, since `mpsc` promises no order among blocked senders. An outbox is
unbounded while a peer takes less than the worker thread produces; the flood test's high
water after the split was 272, and a worker thread with a pending outbox yields rather
than parks, so a saturated pool burns CPU on that thread. A throttle that skips the
client lane while a remote outbox is deep would bound it without touching the deadlock
argument; not built, because nothing asked for it.

**The B8 reorder.** At 110e006 the pool suite failed 3 of 70 runs with
`expected a batch, got NoPocketAvailable`: a publish into a pocket on the order
machine's own worker thread queues its `Enqueue` locally, and a `Pull` already waiting on
the client lane was stepped before it. The fix ends a client-lane burst as soon as a
step has queued a local message, so the local queue is drained before any later lane
item; 70 of 70 after, and 110 of 110 over the probe copies. The guarantee is complete for
a sequential client; a local queue deeper than the burst limit of 64 can still be
overtaken under load, which the protocol tolerates.

**Shown to bite.** For the invariant, fencing and one-thread tests the code was broken
on purpose at 109ddb0, each break restored with `git checkout --` and `git status --short`
empty after the last (`ai_tmp/t20-breaks.md`). That file records no exit status for any
run; the `test result: FAILED` line is the evidence for each break and the
`test result: ok` lines for the restore.

Break 1, the invariant test: `RequeueHeldRecord` in `src/pocket.rs` no longer sets the
record back to `Queued`, so a held record stays held after a release or a lapse.
`cargo test --manifest-path spikes/worker-pool/Cargo.toml --locked --test invariants`:

```text
109ddb0 with uncommitted changes
test every_invariant_holds_after_every_step_of_a_scripted_sequence_with_refusals ... FAILED
thread 'every_invariant_holds_after_every_step_of_a_scripted_sequence_with_refusals' panicked at tests/invariants.rs:233:5:
assertion `left == right` failed: after (45, Tick(Timestamp(13000000)))
  left: [... Violation { invariant: "OutstandingOnlyWhileLeased", detail: "record RecordKey(\"s1\") is Held in pocket PocketKey(\"B\"), which is Queued" } ...]
 right: []
test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Break 2, the fencing tests: the `LeaseNotActive` guard dropped from the shared lease
guard in `src/pocket.rs`. `--test pool` and `--test invariants`:

```text
109ddb0 with uncommitted changes
test an_action_naming_an_ended_lease_is_rejected_under_a_successor_lease ... FAILED
thread 'an_action_naming_an_ended_lease_is_rejected_under_a_successor_lease' panicked at tests/pool.rs:402:52:
called `Result::unwrap()` on an `Err` value: "Ack { consumer: ConsumerId(\"c\"), lease: LeaseId { partition: Partition(0), serial: 0 }, record_key: RecordKey(\"r\") } replied Acked, wanted Rejected(LeaseNotActive)"
test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s

test an_action_naming_an_ended_lease_is_rejected_under_the_successor_lease_and_records_are_unchanged ... FAILED
  left: Acked
 right: Rejected(LeaseNotActive)
test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Break 3, the one-thread test: the static assignment in `src/pool.rs` gives partition
`p` to worker thread `p mod N` and also to the next worker thread, so two worker
threads each own and tick a machine for the same partition. A literal "two threads step
one instance" is unreachable without surgery, since a machine is owned by value in one
worker thread's map and stepped through its `&mut`. `--test pool`:

```text
109ddb0 with uncommitted changes
test each_partition_is_stepped_by_one_thread_and_never_more_than_the_pool_has ... FAILED
thread 'each_partition_is_stepped_by_one_thread_and_never_more_than_the_pool_has' panicked at tests/pool.rs:267:9:
assertion `left == right` failed: Partition(Partition(0))
  left: Some(2)
 right: Some(0)
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
```

After each restore, at 109ddb0 clean, the same targets:

```text
109ddb0
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Measurement

`just spike` with no arguments sweeps (1,1), (1,18), (6,18) and (18,18), worker threads
and partitions, with 18 publishers and 18 consumers (twice the host's
`available_parallelism`, shared by every row so the rows compare), 256 pockets, lease
TTL 200 ms, decay TTLs 1000 ms (the module's 12 h would decay nothing in a 4 s run),
tick 10 ms, queue capacity 1024, 3 s measured after 1 s warm-up, three repeats, the
whole report of the median repeat by publishes a second, or of the first unhealthy
repeat ahead of it, with every row counting its unhealthy repeats and the binary exiting
1 when any row has one. The publishers publish with repeated keys so that supersedes and
stale drops happen; the consumers pull, ack seven of eight, nack one of eight, release
three of four pulls, and consumer 0 sleeps three TTLs after every batch so that lapses
happen. The quotable table, taken at the final code commit 340e8d0 on a clean worktree
(`ai_tmp/t20-sweep-340e8d0.txt`, 12:50:53 PDT), 20 columns with release latency. The
binary exits 1 when any row has an unhealthy repeat, so the exit code 0 recorded on the
file's last line is itself evidence that every repeat of every row was healthy. The note,
thresholds and invariants preamble lines are in the file; the host line, the load line,
the column header and the rows:

```text
commit 340e8d0, worktree clean
worker-pool spike on macOS 26.4; Apple M5 Pro; hw.ncpu 18; perf cores 6; efficiency cores 12; available_parallelism 18; commit 340e8d0
load: 256 pockets, lease_ttl 200 ms, superseded/dead-letter ttl 1000 ms, tick 10 ms, queue capacity 1024, 3.0 s measured after 1.0 s warm-up, median of 3 run(s) by pub/s; clients = publishers+consumers, closed loop
N,P   | clients |    pub/s |   pull/s | ack+nack/s | release/s | p50/p99 pub (us) | p50/p99 pull (us) | p50/p99 ack+nack (us) | p50/p99 release (us) | send-wait p99 pub (us) | retry pulls | msgs local | msgs channel | outbox hwm |   ticks | busy % (max/mean) | lapses | parks | invariants
1,1   | 18+18   |   245792 |    42063 |     158603 |     31549 |           67/119 |            67/129 |                67/119 |               67/117 |                      0 |           0 |     394971 |            0 |          7 |     423 |             19/19 |     14 |     0 |         ok
    bottleneck: outside the machines' steps (no worker thread above 60 % in step; max 19 %): the pool's loop or the clients, not told apart
    requests 1931610 replies 1931610 timed out: 0; publishes new-pocket 0 new-key 468522 superseded 125668 stale 140945 rejected 154; lease-not-active 33; empty pulls 0; unhealthy repeats 0 of 3; wall 4.24 s
1,18  | 18+18   |   244738 |    41878 |     157923 |     31408 |           67/126 |            67/134 |                67/126 |               67/121 |                      1 |           0 |     391505 |            0 |          5 |     425 |             19/19 |     12 |     0 |         ok
    bottleneck: outside the machines' steps (no worker thread above 60 % in step; max 19 %): the pool's loop or the clients, not told apart
    requests 1913816 replies 1913816 timed out: 0; publishes new-pocket 0 new-key 466182 superseded 124871 stale 140437 rejected 164; lease-not-active 34; empty pulls 0; unhealthy repeats 0 of 3; wall 4.26 s
6,18  | 18+18   |   383660 |    43516 |     203517 |     32637 |           35/175 |           113/543 |                33/172 |               33/175 |                      8 |       27573 |      70657 |       384463 |          2 |    2538 |             13/10 |     11 |     0 |         ok
    bottleneck: outside the machines' steps (no worker thread above 60 % in step; max 13 %): the pool's loop or the clients, not told apart
    requests 2626073 replies 2626073 timed out: 0; publishes new-pocket 0 new-key 601078 superseded 244218 stale 301785 rejected 291; lease-not-active 42; empty pulls 0; unhealthy repeats 0 of 3; wall 4.23 s
18,18 | 18+18   |   246626 |    36021 |     148736 |     27016 |           59/228 |           169/446 |                45/188 |               44/184 |                     14 |       19706 |      17423 |       349589 |          2 |    7650 |               6/2 |     12 |     0 |         ok
    bottleneck: outside the machines' steps (no worker thread above 60 % in step; max 6 %): the pool's loop or the clients, not told apart
    requests 1805965 replies 1805965 timed out: 0; publishes new-pocket 0 new-key 439606 superseded 138115 stale 159538 rejected 190; lease-not-active 41; empty pulls 0; unhealthy repeats 0 of 3; wall 4.26 s
exit code 0
```

A second sweep at 340e8d0 (`ai_tmp/t20-sweep-340e8d0-2.txt`, 12:51:51 PDT, exit code 0
on its last line) agrees: every rate within 6 % on every row and column (the worst are
(6,18) pulls and releases a second, 43,516 against 41,047 and 32,637 against 30,786),
every p50 within 6 % (the worst is (6,18) pull, 113 against 120 us), every p99 within
8 % (the worst is (18,18) pull, 446 against 482 us). Every row of both runs: `ok` over
the 23 invariants named in the preamble, violations 0, `timed out: 0`, requests equal
to replies, `unhealthy repeats 0 of 3`, parks 0, lapses 11 to 15. The two sweeps at
ce593ec (`ai_tmp/t20-sweep-ce593ec.txt`, `-2.txt`, both exit code 0) and the two at
3de4bc1 (`ai_tmp/t20-sweep-final.txt`, `-2.txt`, no exit status recorded) are history
now; the (1,18) outlier of the 3de4bc1 run 1 recurred in none of the four later runs,
and (1,18) equals (1,1) within 1 % at 340e8d0, so partitions cost nothing on one worker
thread. These numbers were derived from the two sweep files by the hand-back writer; the
load agent's reading of the 340e8d0 sweeps was not written when these notes were.

What is fastest, per column, both runs agreeing on every ordering stated here: publishes
a second, (6,18) at 367k to 384k, then (18,18), (1,1) and (1,18) tied at 244k to 248k,
within 1.6 % of each other, which the preamble's noise rule says orders nothing; six
worker threads buy 1.5 to 1.6 times one, eighteen buy nothing. Pulls a second do not
rise with worker threads: (1,1), (1,18) and (6,18) all at 41k to 44k, and 14 to 17 %
lower at (18,18), 35k to 36k. Acks and nacks a second, (6,18) at 194k to 204k, then
(1,1) and (1,18) at 157k to 159k, then (18,18) at 147k to 149k. Publish p50 is best at
(6,18), 35 to 36 us, against 57 to 59 at (18,18) and 67 on one worker thread; publish
p99 is best on one worker thread, 119 to 129 us. Pull p50 is best on one worker thread,
67 to 68 us, against 113 to 120 at (6,18) and 169 to 177 at (18,18), the retry round
trips; pull p99 likewise, 129 to 137 us against 543 to 565 and 446 to 482. Release p50
is best at (6,18), 33 to 34 us, against 67 on one worker thread, where a release queues
behind the publishes. Retry pulls are 0 on one worker thread in both rows of both runs,
25.8k to 27.6k with six and 19.7k to 19.8k with eighteen. Against ce593ec every
ordering above is the same except one: there (18,18) led (1,1) and (1,18) by 4 to 6 %
on publishes a second, and at 340e8d0 the three are tied; the rates themselves moved by
at most 8 % ((6,18) publishes, 346k to 357k then, 367k to 384k now).

**The bottleneck, as the fixed binary names it.** Every row's bottleneck line reads
"outside the machines' steps (no worker thread above 60 % in step; max N %): the pool's
loop or the clients, not told apart", with N at 19 on one worker thread, 13 with six and
6 with eighteen at 340e8d0. The caveat is in the preamble: `busy %` is step time only,
time inside `Machine::step` over the pool's wall, so routing, replies, channel
operations, allocation and wake-ups are outside it, and the pool exposes no per-thread
CPU time. The line therefore cannot by itself tell "the pool's loop is the cost" from
"the clients offer too little", which is why checkpoint 3 renamed it from the earlier
"clients" and why the client-scaling run below exists. No row names the channels: the
outbox high water never exceeded 8 against a capacity of 1024 in any sweep, and
send-wait p99 for a publish never exceeded 14 us, so step 6's clause "only if the table
names the channels" did not fire.

**Client scaling**, eight single-repeat runs of 2 s after 1 s warm-up at 340e8d0 on a
clean worktree (`ai_tmp/t20-scaling-340e8d0.txt`, 12:52:43 PDT; each run's exit code is
recorded after its row and the file's last line is
`exit code (worst of the eight runs) 0`). The header and the eight rows, verbatim:

```text
commit 340e8d0, worktree clean
N,P   | clients |    pub/s |   pull/s | ack+nack/s | release/s | p50/p99 pub (us) | p50/p99 pull (us) | p50/p99 ack+nack (us) | p50/p99 release (us) | send-wait p99 pub (us) | retry pulls | msgs local | msgs channel | outbox hwm |   ticks | busy % (max/mean) | lapses | parks | invariants
1,1   | 8+8     |   244398 |    37087 |     149242 |     27816 |            29/54 |             29/56 |                 29/54 |                29/53 |                      1 |           0 |     273126 |            0 |          2 |     303 |             19/19 |      5 |     0 |         ok
1,1   | 16+16   |   238236 |    40304 |     152988 |     30228 |           60/117 |            60/119 |                60/117 |               60/115 |                      1 |           0 |     291034 |            0 |          2 |     303 |             19/19 |      5 |     0 |         ok
1,1   | 32+32   |   223259 |    39730 |     146895 |     29796 |          132/556 |           132/561 |               132/554 |              132/557 |                      1 |           0 |     268892 |            0 |          2 |     303 |             22/22 |      5 |     0 |         ok
1,1   | 72+72   |   237403 |    43293 |     158388 |     32472 |          283/655 |           284/657 |               283/655 |              283/654 |                      1 |           0 |     256794 |            0 |          2 |     304 |             21/21 |      5 |     0 |         ok
6,18  | 8+8     |   267164 |    35017 |     152111 |     26262 |            24/67 |            41/167 |                 24/65 |                24/64 |                     12 |       11096 |      46695 |       229305 |          2 |    1818 |              13/9 |      5 |     0 |         ok
6,18  | 16+16   |   344670 |    39089 |     182557 |     29316 |           34/176 |           111/523 |                32/173 |               32/173 |                     10 |       19989 |      48644 |       266040 |          2 |    1812 |             14/10 |      5 |     0 |         ok
6,18  | 32+32   |   351000 |    36420 |     179234 |     27313 |           44/442 |           339/766 |                42/438 |               41/434 |                      9 |        3933 |      35544 |       220416 |          2 |    1823 |             15/10 |      5 |     0 |         ok
6,18  | 72+72   |   356091 |    40300 |     193491 |     30220 |          72/1038 |          762/1386 |                64/991 |               59/956 |                      7 |         305 |      35143 |       250339 |          2 |    1818 |             16/11 |      5 |     0 |         ok
exit code (worst of the eight runs) 0
```

All eight runs `ok`, violations 0, `timed out: 0`, requests equal to replies,
`unhealthy repeats 0 of 1`, parks 0, lapses 5 (the stalled consumer's). The table
resolves the "not told apart" label. At (1,1) publishes a second are flat, 244k, 238k,
223k, 237k from 8+8 to 72+72 clients, while publish p50 grows linearly with the client
count, 29, 60, 132, 283 us: a closed loop whose throughput stays put while its latency
scales with the offered concurrency is served by a saturated server, so on this host on
this day one worker thread saturates at about 450k client inputs a second (publishes,
pulls, settles and releases summed: 440k to 472k over the four rows) with step time one
fifth of its wall, and the other four fifths is outside the steps: the pool's loop (the
lane `try_recv` bursts, the envelope and reply-channel allocation, the reply `send`, the
wake-ups) and whatever share of the thread's wall the host's scheduling took, which
`busy %` divides by wall time and cannot tell apart; parks were 0 in every row. Not the
clients, and not the machines. At (6,18) publishes a second rise from 267k at 8+8 to
345k at 16+16 and then sit on a plateau, 351k and 356k, while pull p50 grows from 41 to
111, 339 and 762 us and pull p99 reaches 1.4 ms as 32+32 and 72+72 client threads
oversubscribe the 18 cores beside the six worker threads; the sweep's 18+18 point sits on
that plateau. So the clients cap neither (1,1) nor (6,18), the two rows scaled; (1,18)
equals (1,1) within 1 % in the sweeps, and (18,18) was not scaled and is oversubscribed
by construction (below). With six worker threads the limit is the loop plus the
cross-worker-thread retry round trips and the host's oversubscription. The machines are
nowhere near a limit in any row. The scaling runs were re-run at 340e8d0 after the
round 2 fixes (`ai_tmp/t20-scaling-340e8d0.txt`, exit code 0 on its last line); the
earlier runs at 3de4bc1 (`ai_tmp/t20-scaling.txt`) showed the same shape, with the
(6,18) row falling rather than sitting flat past 16+16. A side observation: at (6,18)
retry pulls fall as clients rise (11k, 20k, 4k, 305), because deeper order-machine
queues let the `PocketBecame` on the peer lane land before the consumer's next `Pull`;
scheduling luck, not a fix.

Two more caveats. Every sweep row runs N worker threads plus 36 client threads on 18
cores, so (18,18) is oversubscribed by construction and its numbers say what that
costs, not what eighteen worker threads would do alone. And the warm-up is at full
load, so `busy %`, which divides by the whole pool wall including warm-up and the stop
drain, is slightly diluted, equally for every row. Why no crate was tried: the channels
were never near capacity in any run, the limit is outside the machines' steps, in the
pool's own loop or the host's scheduling of the worker thread and never in the channels,
and step 6 puts a crate behind the interface only when the table names the channels. The
numbers compare configurations on this host on this day and claim nothing else.

Before the B8 fix the (1,1) and (1,18) rows looked worse on the pull side for a reason
that was a defect, not a property of one worker thread. Pre-fix at 110e006 against
post-fix at 110e006 with uncommitted changes, run 1 of each from the load agent's report
(`ai_tmp/t20-sweep.txt` and `t20-sweep-post.txt`, neither of which records an exit
status):

| N,P | pull/s before / after | retry pulls before / after | p50/p99 pull (us) before / after |
| --- | --- | --- | --- |
| 1,1 | 30,478 / 40,068 | 43,154 / 0 | 151/649 and 69/148 |
| 1,18 | 29,856 / 41,172 | 42,653 / 0 | 154/657 and 68/133 |
| 6,18 | 35,080 / 42,246 | 72,233 / 27,443 | 171/762 and 117/529 |
| 18,18 | 32,433 / 32,650 | 31,774 / 17,143 | 183/637 and 201/604 |

Publishes a second were unchanged within 3 % on every row. On one worker thread the pull
path now costs the same as the publish path.

### The boundary

The machines are library-shaped and need no effect trait: nothing in a machine needed a
thread or the clock otherwise than as an input, and `now` reaches a machine only as the
argument of `step`. The order machine, the pocket machine, the protocol, the state, the
partition function, `Direct` and the invariant check would be libpachinko under decision
0003; the pool, which owns the threads, the channels, the clock and the ticks, and the
load generator and the demo binary, would be `pachinko` under decision 0008, behind
effect traits for the clock and threads that need their own decision records first
(Recommendation). Checkpoint 1's reviewer answered "none found" to the question whether
`now` enters a machine any other way than as an input, after searching the machine
sources for clock reads, thread operations, locks, randomness, atomics, sleeps and
spawns.

The grep the brief asked for, at 3de4bc1 with uncommitted changes, lists more files than
the brief predicted, because doc comments say "single-threaded" and the partition module
names `RandomState` to say why it is not used:

```text
$ grep -rln 'Instant\|thread\|Mutex\|RandomState' spikes/worker-pool/src/
spikes/worker-pool/src/pool.rs
spikes/worker-pool/src/partition.rs
spikes/worker-pool/src/machine.rs
spikes/worker-pool/src/protocol.rs
spikes/worker-pool/src/load.rs
spikes/worker-pool/src/load/host.rs
spikes/worker-pool/src/main.rs
spikes/worker-pool/src/load/clients.rs
spikes/worker-pool/src/lib.rs
spikes/worker-pool/src/direct.rs
spikes/worker-pool/src/load/report.rs
spikes/worker-pool/src/pool/worker.rs
exit 0
```

Restricted to lines that are not comments, the hits are only the pool and the load:

```text
$ grep -rn 'Instant\|thread::\|Mutex\|RandomState\|std::thread' spikes/worker-pool/src/ | grep -v '^\S*:[0-9]*:\s*//' | cut -d: -f1 | sort | uniq -c
   2 spikes/worker-pool/src/load.rs
  13 spikes/worker-pool/src/load/clients.rs
   1 spikes/worker-pool/src/load/host.rs
  15 spikes/worker-pool/src/pool.rs
  16 spikes/worker-pool/src/pool/worker.rs
exit 0
```

`src/main.rs` reaches the clock and threads only through `load`. Every hit in `lib.rs`,
`machine.rs`, `protocol.rs`, `direct.rs` and `partition.rs` is a doc comment.

### Crates

None added: the spike's `[dependencies]` table is empty and its lockfile holds one
package. The seven surveyed crates were re-verified on the day (Survey) and none earned
a place, because no measurement named the channels (Measurement). What
`crossbeam-channel` would have bought is its `select!` over several receivers: std has
none, so the pool's two inbound lanes are drained with `try_recv` bursts and the idle
wait is `park_timeout` with `unpark` from every sender, written by hand with a park
token against the check-then-park race and a `OnceLock` per worker thread's `Thread`.
That is the second lane and the wake-up protocol, about sixty lines, and it is the one
place a crate would have shortened the code rather than sped it up. `disruptor` was not
tried: it would change the architecture rather than the plumbing, and nothing in the
tables says the plumbing is the limit, so a before-and-after would have measured a
change the evidence does not ask for. `crossbeam-utils`' `CachePadded` and `Backoff`,
`rtrb` and `flume` fall with the same reasoning. `parking_lot` was never a candidate: no
machine holds a lock, and the non-comment grep above shows none outside the pool and the
load. `foldhash` was replaced by the inline FNV-1a, which the ticket allowed.

### Representation

Every place where the prototype's representation departs from the module's literal
model, each with its argument that no surface can tell, or the statement that one can.
The spike resolves none of them and touches no file under `docs/specs/`; they go to a
`spec-change` ticket (Open points). The list is as revised by checkpoint 1.

1. Two position counters instead of one `queue.next_position`: no surface exposes a
   position; records order only within a pocket, pockets only among pockets.
2. Pocket status held twice: authoritative in the pocket machine, a lagging view in the
   order machine, which answers `Pull` from its view. This departure is observable and
   has no unobservability argument; it is the window the Context names. Two
   observables, both through `Pull`'s response: (a) `NoPocketAvailable` while a pocket
   is queued in its pocket machine and its `Enqueue` (new, revived) or `PocketBecame`
   (released at the back) is in flight, after the client already holds the `Published`
   or `Released` reply; (b) after `LeaseExpiryKeepsPocketAtFront`, a `Pull` processed
   before the `PocketBecame { RequeuedKeepingPosition }` claims the next queued pocket
   instead of the front one, so the batch's pocket key names B where the module's
   `min_by(free, position)` names A. Neither loses a pocket or double-claims
   (`spike:OrderViewConsistent` and `spike:OrderMatchesViews` hold at quiescence); the
   front pocket is served by the next pull. Shown by hand-stepping the machines at
   checkpoint 1 (`ai_tmp/review/T20-1/probe-conformance/`); `tests/window.rs` covers a
   different window, the same consumer's pull racing its own lapse or release, and
   `tests/invariants.rs` cannot reach either observable because `Direct` runs each
   submit to quiescence and the oracle's `free` is built from the same lagging view. The
   pool-level test that exhibits (a) or (b) and asserts that the next pull claims the
   front pocket is still owed (Open points). Closing the window costs one hop per publish
   and release (the reply via the order machine) or a different decomposition.
3. The clock is a tick: a lapse is detected at the first tick after `expires_at`; the
   surface exposes `expires_at`, not the lapse instant; `LeaseLapses`'
   `expires_at <= now` holds at it. Timestamps are `u64` nanoseconds since the pool
   started and `plus` and `since` saturate rather than wrap, so `expires_at = now + ttl`
   is exact until about 584 years after start, which no run reaches; at the edge a TTL
   is shortened, which the surface would show as an early lapse.
4. `Lease` has an identity, `LeaseId { partition, serial }`; the module's lease has none.
5. Publishers are recorded only in the order machine, and no publish checks that the
   publisher joined, as in the module.
6. A publish with `rank` equal to the stored rank is `Rejected(EqualRank)`, the
   checker's placeholder for open question 1.
7. Actions from parties that never joined are accepted: no rule requires the join, and
   the surfaces' `facing` implies it.
8. Ended leases are kept forever in the pocket, so memory grows per lease; an
   implementation prunes once nothing can name them. Unobservable.
9. `limit` is `u64`; 0 stands for the module's "not positive".
10. `Rank`, positions and the two counters are unsigned 64-bit and `consecutive_expiries`
    is `u32`, where the module's `Integer` is unbounded and admits negatives. A negative
    rank is an input the spike cannot be given; over every input it accepts the
    comparisons `rank > record.rank` and `held_rank < rank` are the module's, so no
    response or exposed `held_rank` differs. An implementation chooses its rank type;
    `propagate` on the module would generate a negative-rank case that a `u64` rejects at
    the type.
11. An idle or parked pocket has no position in the order machine's view, where the
    module keeps its last one. No rule reads it in those states (`min_by` ranges over
    queued pockets; revival assigns a fresh position), positions are never reused, so no
    surface can tell, and `UniquePocketPosition` and `PocketPositionsBelowNext` are
    checked over the queued and leased views, the only ones that carry a position.

### Recommendation

A proposal. The maintainer takes the decision; nothing here is taken. After each
sentence of the drafted record the measurement that supports it is named in brackets.

Drafted record text, status "proposed, the maintainer's to take":

> **Decision 00NN: Single-threaded machines over partitions on a bounded pool of worker
> threads**
>
> **Context.** Decision 0014 deferred the service's runtime until a specification named a
> surface and a spike compared the options. The root module names two surfaces, and
> ticket T20 built and measured the execution model the maintainer proposed: every rule
> of the module as a branch of a single-threaded state machine that takes `now` as a
> value, the pockets' records and leases in pocket machines over partitions of pockets
> and the pocket order and the lease routing in one order machine (decomposition B), on
> a bounded pool of long-lived worker threads under `std::thread::scope` with bounded
> lanes, outboxes and a tick. The spike's machines needed no effect trait [the boundary
> grep and checkpoint 1's "none found"], every rule ran as a branch with the module's
> invariants checked after every step [the scripted invariants test; 23 invariants `ok`
> in every measured row], the lease identifier acted as the fencing token [the three
> fencing tests and break 2], and the pool never blocked a worker thread on a send
> [the flood test at capacity 1 with both lanes full]. On the host the machines' steps
> took a fifth of a saturated worker thread's wall and the rest was outside them, in the
> pool's loop or the host's scheduling, not in the clients [the client-scaling table at
> (1,1): flat throughput, latency linear in the clients]; six worker threads bought 1.5
> to 1.6 times one for publishes and nothing for pulls, and eighteen bought nothing [the
> two sweeps at 340e8d0; the same ordering at ce593ec and 3de4bc1]; one worker thread
> gave the lowest pull latency, by 1.7 times over six worker threads and 2.5 over
> eighteen [the same sweeps, pull p50].
>
> **Decision.** The service runs the domain as single-threaded machines over partitions
> of pockets, decomposition B, on a bounded pool of long-lived worker threads whose
> count is configuration, with the pocket order and the lease routing in one order
> machine; the machines are library code with `now` as a value and the pool is service
> code behind effect traits for the clock and threads. Outside the machines' steps, not
> in them, is where the next effort goes if more throughput is wanted [the scaling
> table; the spike reports step time over wall time only].
>
> **Consequences.** The window between a pocket machine's decision and the order
> machine's hearing of it is observable through `Pull` [Representation item 2], and the
> specification must say which reading of `free` it means before the implementation
> ticket starts [Open points, the C1 question]. Positions come from two single-writer
> counters, which the specification must either adopt or rule out [Representation item
> 1]. Partitions cost nothing on one worker thread [the (1,1) against (1,18) rows] and
> the thread count is a deployment choice, so the first deployment may run one worker
> thread and keep the best pull latency, at the cost of a third of the publish
> throughput, a fifth of the ack and nack throughput and 1.9 to 2 times the publish and
> release p50 on a loaded host [the (1,1) against (6,18) rows at 340e8d0 run 1: 246k
> against 384k publishes a second, 159k against 204k acks and nacks a second, publish
> p50 67 against 35 us, release p50 67 against 33 us, pull p50 67 against 113 us]. The
> transport half of the spike has not run; this record settles the execution model
> alone, or waits for it (Open points).

Implementation-ticket outline, as a proposal:

- Start from the module and `just plan-spec pachinko`, not from this code; the spike's
  README says it is a prototype the implementation learns from, never its first commit.
- The machines go into libpachinko as plain values with `now` as a value, in the shape
  the spike found (a step function per machine, one enum of inputs and one of outputs,
  a lease identifier that carries its partition), with the tests `propagate` derives
  from the module; the spike's invariant oracle is the pattern for the test that asserts
  every invariant after every step.
- The pool goes into `pachinko` behind effect traits for the clock and for threads,
  which are new kinds of effect and need decision records first under decisions 0003 and
  0008; the fakes beside them make the pool's tests deterministic where the spike's rely
  on sleeps.
- Open questions to settle before the ticket is written: the Release-then-Pull window
  (close it with the extra hop, or specify the stream-order reading); A versus B
  revisited with the retry numbers in hand (B's two extra hops on a pull after a release
  or a lapse against A's hop on every consumer action); and whether the superseding record
  waits for the transport half.

### Claims

The table's Outcome column is filled above. Claim 3, read on 2026-10-31 from
`https://docs.renovatebot.com/modules/manager/cargo/`: "By default, Renovate will check
any files matching the following regular expression: `/(^|/)Cargo\.toml$/`" and
"Extracts dependencies from `Cargo.toml` files, and also updates `Cargo.lock` files too
if found." `spikes/worker-pool/Cargo.toml` matches, so only the `rust-version` regex
manager needed the path (`renovate.json` line 109). Holds; the dashboard shows it only
after the merge.

Claim 5 holds. Timed at 340e8d0 on a clean worktree
(`ai_tmp/t20-clippy-timing-340e8d0.txt`, 12:53:34 PDT) on the host, with the workspace
and fuzz targets already warm, so "cold" means the spike's own target only, removed
before each cold run; all four exit 0. The zsh time lines as the file holds them:

```text
commit 340e8d0, 0 uncommitted paths
$ rm -rf spikes/worker-pool/target; time just clippy   (cold)
cargo clippy --manifest-path spikes/worker-pool/Cargo.toml --all-targets --locked -- -D warnings
    Checking worker-pool v0.0.0 (.../spikes/worker-pool)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.51s
just clippy  1.12s user 0.31s system 173% cpu 0.828 total
exit=0

$ time just clippy   (warm)
just clippy  0.09s user 0.06s system 84% cpu 0.178 total
exit=0

$ rm -rf spikes/worker-pool/target; time cargo clippy --manifest-path spikes/worker-pool/Cargo.toml --all-targets --locked -- -D warnings   (spike line alone, cold)
cargo clippy --manifest-path spikes/worker-pool/Cargo.toml --all-targets  --   1.09s user 0.24s system 223% cpu 0.593 total
exit=0

$ time cargo clippy --manifest-path spikes/worker-pool/Cargo.toml --all-targets --locked -- -D warnings   (spike line alone, warm)
cargo clippy --manifest-path spikes/worker-pool/Cargo.toml --all-targets  --   0.03s user 0.02s system 81% cpu 0.056 total
exit=0
```

The spike adds under a second to a cold `just clippy` and about nothing to a warm one,
against T16's 7.87 s cold and 0.06 s warm for `fuzz/`. The earlier timing at ce593ec
(`ai_tmp/t20-clippy-timing.txt`) read 1.229 s cold and 0.181 s warm.

### Adversarial reviews

Harness at every checkpoint: Codex (codex-cli 0.165.0) with model gpt-6-astra, read-only
and ephemeral, the harness the executing agent is not. Codex's sandbox had rustc 1.96 and
could not build the crate (`worker-pool@0.0.0 requires rustc 1.99`), so every review is
source inspection; the harness reran the test and clippy commands on the pinned 1.99.1
where a brief asked for them and appended the output unaltered. Triage and apply were
separate Claude Fable 5.1 subagents, neither the author nor the reviewer, each with a
fresh context; every "the test survives mutation X" claim was reproduced in a scratch
copy of the crate under `ai_tmp/` before a verdict. Files: `ai_tmp/review/T20-<n>/`
with the briefs, `findings-*.md`, `verdicts*.md` and `applied.md`.

**Checkpoint 1, the machines** (ticket step 4), over e4f9393 plus the uncommitted pool
files, two lenses.

| Id | Severity reviewed | Verdict | What happened |
| --- | --- | --- | --- |
| C1 | high | VALID-DOC, re-rated medium, plus a VALID-HANDBACK question | Pull reads the lagging view: both window scenarios reproduced in a probe; Representation item 2 rewritten to say it is observable; the linearizable-versus-stream-order question handed back. |
| C2 | medium | VALID-DOC, re-rated low | `Rank(u64)` narrows the module's `Integer`: Representation item 10. |
| C3 | low | VALID-DOC | Saturating `u64` timestamps: Representation item 3 extended. |
| C4 | low | VALID-DOC | Idle and parked views drop the position: Representation item 11 and a doc comment on `view_position`. |
| B1 | medium | VALID | The "pipelined" test never pipelined, since `Direct` drains to quiescence: `tests/window.rs` added with four hand-stepped windows; the test renamed and its vacuous assertion dropped. |
| B2 | medium | VALID | The consistency check never read the pocket order a pull pops: `spike:OrderMatchesViews` added to `invariants::check`, two negative unit tests; the mutation that dropped position 6 passed before and fails now. |
| B3 | low | VALID | Outcome coverage was not guard coverage: refusals tallied per action and reason, seven missing refusals added to the script, the floor raised from 14 to 21 (the run tallies 23). |
| B4 | high | VALID-DOC, re-rated low | e4f9393 declares `pub mod pool;` one commit before 110e006 adds it, a bisect hazard, not a defect at HEAD; the environment leg (rustc 1.96) INVALID. Commit discipline recorded under Open points. |

Nothing handed back needed a code change inside the machines. The apply report's gates,
at 110e006 with uncommitted changes; the fixes landed in 109ddb0:

```text
110e006 with uncommitted changes
cargo clippy --manifest-path spikes/worker-pool/Cargo.toml --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
clippy exit 0
cargo test --manifest-path spikes/worker-pool/Cargo.toml --locked --lib --test invariants --test replay --test window
test result: ok. 149 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test exit 0
```

**Checkpoint 2, the pool** (ticket step 5, the maintainer's added checkpoint), over
110e006, two lenses.

| Id | Severity reviewed | Verdict | What happened |
| --- | --- | --- | --- |
| C1 | medium | VALID | The renewal half never sent a late renewal (TTL 200 ms, renewals at 1 and 31 ms): rewritten with TTL 30 ms, a 2 s tick, a 100 ms sleep, the late renewal accepted and zero ticks asserted. |
| C2 | medium | VALID, re-rated low | `double_replies` counted only a full channel: the reply handle now carries an answered flag shared across forwards, with a unit test. |
| C3 | low | VALID-DOC | 73 prose lines said bare "worker" for the thread: every one now says "worker thread"; identifiers stay. |
| B1 | medium | VALID | A send admitted between a worker thread's exit decision and its receiver drop was lost and leaked its token (probe: a hang with two worker threads): `send` re-reads `stop` after the increment and refuses with `PoolStopped`. |
| B2 | medium | VALID, re-rated low | Quiescence did not wake worker threads waiting to stop (probe: 2.01 s against 19 ms): the last token's release after stop wakes everyone. |
| B3 | medium | VALID-DOC, re-rated low | The thread id is the worker thread's self-report taken outside `step`; the single-stepper guarantee is structural and the docs now say so. |
| B4 | low | VALID | The fencing test did not check payloads: two assertions added. |
| B5 | medium | VALID | The flood test passed at capacity 100,000: a `lane_full` counter on both `Full` arms, asserted positive on every worker thread. |
| B6 | medium | VALID, adapted | The 10 s bound could not end a deadlock (probe: a hang): a watchdog thread around `Pool::start` in the flood test and the panic half. |
| B7 | medium | VALID, re-rated low, subsumed by C1 | "Late renewal wins" was not exercised. |
| B8 | medium | VALID, re-rated high | Not the listed sleep margins but a reorder: 3 of 70 runs failed at tests/pool.rs:554 with `expected a batch, got NoPocketAvailable`; the client-lane burst now ends when a step queued a local message, with a unit test; 70 of 70 after. |
| B9 | medium | VALID, re-rated low | Same defect as C2, same fix. |

Handed back: none. No fix touched the protocol or a machine. The apply report's result
lines, at 110e006 with uncommitted changes, after 30 of 30 runs of the pool target
passed; the report records no exit status for the clippy or test run, so the `Finished`
and `test result` lines are the evidence; the fixes landed in 109ddb0:

```text
110e006 with uncommitted changes
cargo test --manifest-path spikes/worker-pool/Cargo.toml --locked
test result: ok. 147 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**Checkpoint 2, round 2** (T20-2b), over the uncommitted fix diff on 110e006, because
B8 was high; both lenses.

| Id | Severity reviewed | Verdict | What happened |
| --- | --- | --- | --- |
| C1 | low | VALID-DOC | Five bare "worker" remained in a `Display` string, two docs and two assertion messages: five word edits. |
| B10 | low | VALID | The stop refusal in `send` released its token without a wake, so a stop could wait out a tick (probe: 0.51 s against at once): the refusal wakes everyone when it released the last token, with a unit test. |
| B11 | low | VALID | No test told fullness from counting attempts: `only_a_full_lane_counts_as_fullness` added (three `try_send`s, one `Full`). |
| B12 | low | VALID | The double-reply test cloned the handle by hand: it now obtains the forwarded handle through a real `Forward`. |
| B13 | low | INVALID | Moving the local drain after the client drain does not break the property, because the loop is a cycle; the burst-break is the fix and the existing test catches its removal. |

Handed back: none. The harness's five reruns of the pool target on 1.99.1 all passed
(0.35 to 0.36 s); its `--all-targets` clippy run failed only on the then-untracked
`tests/window.rs`, another agent's in-progress file, fixed by its owner before the apply
phase, whose every gate was green after each of the four fixes (pool unit tests 12, pool
target 6, ten runs of ten). The fixes landed in 109ddb0.

**Checkpoint 3, the measurement and the handbook** (ticket steps 6 and 7, the
maintainer's added checkpoint), over 109ddb0 and the post-fix sweeps, two lenses.

| Id | Severity reviewed | Verdict | What happened |
| --- | --- | --- | --- |
| C1 | medium | VALID | Ack, nack and release had no latency and release no rate: `release/s` and ack+nack percentiles added; the sweep re-run. |
| C2 | low | VALID, adapted | A passing row named no invariant: the preamble now lists the 23 names, with a unit test that reads `invariants.rs` and asserts set equality. |
| C3 | medium | VALID, raised to high, same defect as B1 | The bottleneck line said "clients" whenever step-busy was under 60 %; the probe refuted it (flat throughput, latency linear in the clients). |
| C4 | low | VALID-HANDBACK | `docs/reference/configuration.md` rows 166 and 167 omit the spike's `rust-version` and lockfile; outside Files touched (Open points). |
| C5 | low | INVALID | The index row says done one commit before the frontmatter, which is the ordering the ticket prescribes; resolved by `status: done` in this commit. |
| C6 | low | VALID | Output strings said bare "worker": every printed string and doc comment now says "worker thread"; the `--workers` flag stays. |
| B1 | medium | VALID, raised to high | The bottleneck label; the line now reads "outside the machines' steps ... not told apart" and the preamble defines `busy %` as step time only. |
| B2 | high | VALID, re-rated medium | A client panic was swallowed by `unwrap_or_default` at the join (probe: exit 0 and `ok`): the panic is re-raised at the join and unwinds through `Pool::start`; the quoted runs' stderr files hold only cargo's lines, which is the evidence no client panicked. |
| B3 | medium | VALID, re-rated low | The window divisor was the configured duration, not the measured window (1.4 to 10 ms short of 1 s): the tally records the real window. |
| B4 | medium | VALID-DOC | "Twice the parallelism cannot be what caps the throughput" was an overclaim: reworded; the client-scaling run is the evidence. |
| B5 | low | VALID-DOC | "Same ranking in every column" was false for pull/s between the two post-fix runs: the Measurement section states agreement per column. |
| B6 | high | VALID | The median dropped a failed repeat (probe: exit 0 and `ok` with a failed invariant in repeat one): the first unhealthy repeat is kept ahead of the median, with unit tests. |
| B7 | low | VALID-HANDBACK | Same omission as C4, row 167. |

Handed back: C4 and B7 (the configuration page), and the re-run of the sweep at the
final commit, which the triage required regardless. The apply report's result lines, at
109ddb0 with uncommitted changes; the report records no exit status for the test run,
so the `test result` lines are the evidence; the fixes landed in 3de4bc1:

```text
109ddb0 with uncommitted changes
cargo test --manifest-path spikes/worker-pool/Cargo.toml --locked
test result: ok. 153 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

No `cargo test` output was recorded at 3de4bc1 on a clean worktree; the two sweeps and
the scaling runs at 3de4bc1 clean are the evidence at that commit.

**Checkpoint 3, round 2** (T20-3b), over the 3de4bc1 fix diff, both lenses in one brief,
because B6 and B1/C3 were high.

| Id | Severity reviewed | Verdict | What happened |
| --- | --- | --- | --- |
| B1 | medium | VALID | A second unhealthy repeat stayed hidden and the binary exited 0: every row counts its unhealthy repeats (`unhealthy repeats K of R`), and the binary exits 1 after the table when any row has one. |
| B2 | low | VALID | `measuring` was set before the window's start timestamp: `opened` is taken first. |
| C1 | medium | VALID | Release latency was still unreported: a `p50/p99 release (us)` column, 20 columns now. |

No high finding, so no third round. All three were applied in the load files and the
README only (`ai_tmp/review/T20-3b/apply-report.md`), which records exit 0 for
`just spike-test`, `just fmt-check`, `just clippy`, `just deny`, `just lock-check` and
`just lint`, and states B1's limitation: a row still prints one run's details, a second
unhealthy repeat is counted on the row's line and fails the binary with exit 1, and its
own invariant names are seen only by a `--repeats 1` rerun. The binary's exit code is
now evidence: 0 means every repeat of every row was healthy. The gate logs beside the
report ran at 3de4bc1 with uncommitted changes, landing in ce593ec:

```text
3de4bc1 with uncommitted changes
cargo clippy --manifest-path spikes/worker-pool/Cargo.toml --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.56s
cargo test --manifest-path spikes/worker-pool/Cargo.toml --locked
test result: ok. 155 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**Checkpoint 4, the whole diff and these notes** (ticket step 8's marked checkpoint 2),
over `git diff main...HEAD` and the hand-back as committed at 913f1b1, two lenses; the
same harness, model and procedure as above. Briefs under `ai_tmp/review/T20-4/`,
findings in `findings-conformance.md` and `findings-breakage.md`, verdicts in
`verdicts-conformance.md` and `verdicts-breakage.md`, the code fixes in
`apply-code-report.md`. The reviewer ran the protected-path diffs (`crates`,
`docs/specs`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `docs/decisions`,
`pyproject.toml`, `docs/manifest.yml`, `CHANGELOG.md`, `.github`) at 913f1b1 and found
only 0016's lockfile count; terminology leaks: none found.

| Id | Severity reviewed | Verdict | What happened |
| --- | --- | --- | --- |
| C1 | low | VALID | Six quoted blocks and two tables rested on files that record no exit status, or no commit, without the notes saying so: five clauses now say "records no exit status" or "records no commit; first committed in 110e006"; nothing invented. |
| C2 | medium | VALID | The client-scaling table was taken at 3de4bc1 and the notes said the round 2 fixes did not touch the measurement path, which was false (two edits in `clients.rs` on the counting path): the eight runs were re-taken at 340e8d0 with the exit status recorded, the quote replaced and the "not re-measured" sentence dropped. |
| C3 | medium | VALID | B's retry path costs four hops when the retry claims a pocket and three when none is queued, not "2, or 3": the hop row names `RetryPull` and the fresh `Claim` and counts 4 or 3; the outline says "two extra hops". |
| C4 | medium | VALID | A's paragraph charged renew a pocket operation the rule does not make, and omitted the joins, the no-pocket pull and the decays: renew is order alone, 1 hop; the four rows added in prose. |
| C5 | medium | VALID | "The other four fifths is the pool's loop" apportioned what `busy %` (step time over wall time) cannot: weakened to "outside the steps: the pool's loop or the host's scheduling, not told apart" in five places, and "the clients cap no row" scoped to the two rows scaled. |
| C6 | medium | VALID | Three quantitative summaries overstated the runs: (1,1) and (1,18) are a tie; "no number moved by more than 5 %" was false (up to 10 %, 27 % for the 3de4bc1 outlier); the record's "lose nothing but publish throughput" now lists what (1,1) loses against (6,18). |
| B1 | medium | VALID | The notes quoted only the 364d921 baseline, not a post-change `just check` nor the network-denial run with its two controls: the "Gate" section above quotes both at 340e8d0. |
| B2 | low | VALID | The median repeat was chosen by raw publish count while every promise says publishes a second, and the divisor is each repeat's own window: `select` now sorts by `publishes_per_second` with `total_cmp`, with a pinning test, fixed in 340e8d0. |

No high finding, so no second round. Handed back: none. The re-run evidence the fixes
made stale (the sweep, the scaling runs, the clippy timing and the gate) is quoted at
340e8d0 above.

**Code review** (`/code-review medium main` over `git diff main...HEAD` at 913f1b1, the
Claude Code code-review skill on Claude Fable 5.1, high-effort recipe, a different
harness from the checkpoints): findings in `ai_tmp/review/T20-cr/findings.md` (R1 to
R10), verdicts in `ai_tmp/review/T20-cr/verdicts.md`, the fixes in
`ai_tmp/review/T20-4/apply-code-report.md`. The bar: VALID-FIX is a real defect against
the spike's promises or the gates with a fix of a few lines; hardening or performance
preferences on a throwaway crate are VALID-DOC, one sentence each under Open points.

| Id | Severity reviewed | Verdict | What happened |
| --- | --- | --- | --- |
| R1 | medium | VALID-FIX | A client-built lease identity naming a partition the pool lacks was routed by modulo, counted as misrouted and never answered, breaking the one-reply promise: `route()` folds the index into the pool's range so the pocket machine answers `UnknownLease`; a unit test and a `Direct` test. |
| R2 | low | VALID-DOC | A zero tick interval makes every worker thread spin; the implementation rejects it (Open points). |
| R3 | low | VALID-FIX | The three unit constructors in `time.rs` multiplied without saturating while `plus` and `since` saturate: `saturating_mul` in each, with a test at `u64::MAX`. |
| R4 | low | VALID-DOC | A refused `Claim` would strand its consumer under a lease no pocket machine granted; unreachable by construction, the implementation rolls back or notifies (Open points). |
| R5 | low | VALID-DOC | The load counts a refused send and a dropped reply handle as a timeout; no completed run can show it (Open points). |
| R6 | low | VALID-FIX | The outbox drain's `Disconnected` arm dropped an envelope without releasing its in-flight token, unlike the other arm: one line, with a unit test. |
| R7 | low | VALID-FIX | `Direct::new` built zero pocket machines for zero partitions where the pool clamps to one: clamped, with a test. |
| R8 | low | VALID-DOC | The outbox high-water mark is an O(worker threads) scan per message (Open points). |
| R9 | low | VALID-DOC | The nanosecond and `usize` saturating conversions are written four times (Open points). |
| R10 | low | VALID-FIX | `docs/operations/maintenance.md` and `renovate.json` counted the toolchain's places as three where five files take their version from the image: "all five" and "five places". |

All five VALID-FIX findings and checkpoint 4's B2 landed in 340e8d0; the apply report
records exit 0 for `just spike-test`, `just fmt-check` (after `just fix` reflowed one
closure), `just clippy`, `just deny`, `just lock-check` and `just check-docs`, and the
Renovate configuration validator did not run because it reaches the network and the
change is to a description string only. No VALID-FIX changed a machine's step semantics.

## Open points

For the maintainer, carried from the writing of this ticket on 2026-10-31:

- Whether the spike's tests join `just check` and CI, or stay outside like the fuzz
  crate's run. The ticket's default is outside: `check` lints the spike through `clippy`,
  which compiles its tests, so rot shows as a build failure, and the gate stays as fast as
  claim 5 measures. If the maintainer wants them inside, the recipe joins
  `pyproject.toml`'s list and `ci.yml`'s `rust` job in a `main` pull request.
- The words "worker thread", "partition", "machine" and "pool" as repository words in
  `docs/project/terminology.md`, if they survive into the handbook. The hand-back proposes
  the rows; the maintainer accepts or renames.
- The spike's lifetime: deleted when the implementation lands, or kept under `spikes/` as
  the reference the implementation ticket cites.
- Whether the record that supersedes 0014 settles the execution model alone, which is this
  ticket's scope, or waits for the second half of the spike, the runtime and transport
  comparison the surfaces trigger.
- The "Representation" questions go to a `spec-change` ticket once the maintainer has read
  them; this ticket neither answers them nor edits the module.

Added at hand-back on 2026-10-31:

- The branch name. The worktree ran on `tickets/t20-worker-pool-spike-complete`, not the
  `ticket/t20-worker-pool-spike` the frontmatter names; kept as prepared. The maintainer
  renames the branch before the pull request or accepts the deviation, as for T16.
- The spike's tests outside `check`, restated with the numbers: `just spike-test` runs in
  about a second of test time on the host and the clippy line of `check` compiles every
  test target, so rot shows as a build failure; the ticket's default stands unless the
  maintainer wants the tests in CI.
- Proposed terminology rows for `docs/project/terminology.md`, repository words, for the
  maintainer to accept or rename:
  - Worker thread: one of a fixed number of threads a pool starts and joins, never
    spawned per request, each stepping the machines of its partitions. Always "worker
    thread", never a bare "worker", which the Consumer row forbids.
  - Partition: a disjoint set of pockets chosen by a stable function of the pocket key,
    whose state is touched by exactly one worker thread. Never a pocket, and a pocket is
    never a partition; a partition holds many pockets.
  - Machine: a single-threaded state machine, a value and a step function that takes one
    input and the current time as a value and returns outputs, with no clock read, thread,
    lock or randomness inside. Library code under decision 0003.
  - Pool: the worker threads, the bounded lanes between them, the clock that stamps each
    input, the tick and the start and stop: everything the machines leave outside
    themselves. Service code under decision 0008.
- The spike's lifetime: deleted when the implementation lands, or kept under `spikes/` as
  the reference the implementation ticket cites. The notes above cite it by path, so a
  deletion would need the hand-back to stand on its own, which it does.
- Whether the record that supersedes 0014 settles the execution model alone or waits for
  the transport half of the spike. The drafted record under Recommendation is written to
  settle the execution model alone and says so.
- The Representation items go to a `spec-change` ticket: item 1 (two counters), item 2
  (the observable window), item 4 (a lease identity), item 6 (equal rank, open question
  1), item 7 (unjoined parties), item 10 (bounded integers) and item 11 (positions of
  idle and parked pockets) each want a sentence in the module or a ruling that the
  implementation may differ.
- Stale handbook text outside Files touched, found by the reviews and not edited:
  `docs/reference/configuration.md` rows 51 and 74 (the fuzz crate's description and the
  `.gitattributes` row name the fuzz lockfile and not the spike's) and rows 166 and 167
  (the toolchain row lists `rust-version` in the root and fuzz manifests only; the Rust
  dependencies row lists `Cargo.lock` and `fuzz/Cargo.lock` only, where
  `spikes/worker-pool/Cargo.toml` and `spikes/worker-pool/Cargo.lock` now carry a pin
  each); decision 0007's sentences that count the lockfiles and name `fuzz/Cargo.lock` as
  the only separate graph; and decision 0014's Context, which says the repository has "no
  domain" and one placeholder module, which T19 made untrue. The maintainer authorises
  those edits or leaves the records as history.
- `.markdownlint-cli2.jsonc`'s ignores list names `target` and so matches only the root
  `target/`: a `cargo doc` run in the spike generated
  `spikes/worker-pool/target/doc/static.files/*.md` and made `just check-docs` fail until
  that directory was removed. An ignore for `spikes/*/target` is a gate fix for the
  maintainer.
- The C1 question from checkpoint 1, for the `spec-change` ticket: the module evaluates
  `free` and `min_by` over the whole state in one step and does not say whether a `Pull`
  must see every publish and release whose reply has already been delivered (a
  linearizable reading) or only those the queue has processed (a stream-order reading).
  The spike implements the second (Representation item 2); the first costs the Pull path
  an extra hop or a different decomposition.
- The pool-level test that exhibits Representation item 2's observable (a) or (b) through
  the pool and asserts that the next pull claims the front pocket, asked for by checkpoint
  1's verdict, was not written; `tests/window.rs` shows the mechanism by hand and the load
  shows the retry path under load, but no pool test asserts the front-pocket recovery.
- Commit discipline: e4f9393 does not build on its own, because `lib.rs` declared
  `pub mod pool;` one commit before 110e006 added the pool module; a bisect hazard, not a
  defect at HEAD. Each step's commit must carry every module its `lib.rs` declares; the
  check is `git ls-tree -r --name-only HEAD spikes/worker-pool/src` against the
  `pub mod` lines.

From the code review (R2, R4, R5, R8 and R9, VALID-DOC), for the implementation ticket:

- `PoolConfig::with_tick_interval` and `--tick-ms` accept zero, which makes every worker
  thread tick on every iteration and `park_timeout(0)` in `idle`, a full spin whose busy
  and tick columns mean nothing; the spike's runs all used 5 ms or more. The
  implementation validates its configuration (a positive tick, positive TTLs) and rejects
  a zero rather than clamping it, since a 1 ns floor still spins.
- A refused `Claim` strands its consumer: the order machine records the lease and the
  `Leased` view before the `Claim` is confirmed, and the pocket machine answers a `Claim`
  on a pocket that is absent or not queued with `Rejected(Protocol)` and a counted
  violation, with no `PocketBecame`, so from then on every `Pull` by that consumer is
  forwarded under a lease no pocket machine granted and answered `Rejected(Protocol)`,
  and the pocket never re-enters the order. Unreachable in the spike (a pocket is queued
  in its machine before the `Enqueue` or `PocketBecame` that puts it in the order, every
  such message rides one FIFO lane, and once queued it leaves that status only by the one
  `Claim` sent for its one order entry), and `tests/window.rs` and the sweep show zero
  violations. The implementation either has the pocket machine answer a refused `Claim`
  with a notification that rolls back the routing and the view, or records the lease
  only when the pocket machine confirms it, at one more hop per claim; it does not leave
  a protocol violation as a silent strand.
- The load's tally counts a refused send and a dropped reply handle as `timed_out`; in
  the spike neither reaches a printed row, because the only mid-run cause is a worker
  thread panic that `Pool::start` re-raises, but the implementation's client keeps
  `Timeout`, `NoReply` and `Send` apart, as `CallError` already does, so a lost worker
  thread is never reported as a client timeout.
- `enqueue` recomputes the sum of every outbox's length to update `outbox_high_water`,
  an O(worker threads) walk per message; the implementation keeps a running count of
  waiting messages, decremented where the outboxes pop, and the sweep is the place to
  show whether it matters.
- The nanosecond and `usize` saturating conversions are written four times (twice in the
  load, twice in the worker thread's loop); the implementation gives `libpachinko`'s
  duration type one saturating conversion the service side imports.
