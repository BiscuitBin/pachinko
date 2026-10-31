---
id: T20
title: "Execution-model spike: bounded worker threads running single-threaded state machines over partitions"
status: todo
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
| 1 | A crate under `spikes/` with an empty `[workspace]` table is its own root, so every `--manifest-path` line reaches it and the root `Cargo.toml` is untouched; `tests/fixtures/effects-violation/Cargo.toml` is the precedent. | Step 3 | |
| 2 | `cargo deny --manifest-path spikes/worker-pool/Cargo.toml --config deny.toml --locked check licenses bans sources` applies the root policy to the spike's graph. | Step 3 | |
| 3 | Renovate's cargo manager finds `spikes/worker-pool/Cargo.toml` by its default file pattern, so only the `rust-version` regex manager in `renovate.json` needs the path; the manager's documentation (`https://docs.renovatebot.com/modules/manager/cargo/`) is read on the day, because the dashboard shows it only after the merge. | Step 7 | |
| 4 | `std::sync::mpmc` is unstable on 1.99.1 and `std::sync::mpsc::sync_channel` and `std::thread::scope` are stable, so the first cut needs no dependency. Verified 2026-10-31 while writing this ticket, with a one-line probe. | Step 2 | |
| 5 | The clippy pass over the spike inside `just check` costs an acceptable time; T16's precedent for `fuzz/` was 7.87 s cold and 0.06 s warm. | Step 9 | |

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

Written by the executing agent, in the sections step 8 names, each quoted output headed
by the commit it ran at and showing its exit status (`CONVENTIONS.md` §11).

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
