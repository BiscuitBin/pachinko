---
id: T20
title: "Pachinko prototype: the queue in libpachinko, Arrow records and a Flight API in the service"
status: todo
depends_on: [T19]
parallel_with: []
branch: ticket/t20-pachinko-prototype
estimated_size: XL
---

# T20: Pachinko prototype: the queue in libpachinko, Arrow records and a Flight API in the service

## Context

T19 wrote the domain down: `docs/specs/pachinko.allium` specifies pockets, records, ranks,
leases, expiry, parking and decay, with two surfaces, `Publishing` and `Consuming`, and
190 test obligations under `just plan-spec pachinko`. Nothing implements it. `libpachinko`
holds one item, `version()`; `pachinko::run` writes one line; decision 0014 defers the
service's runtime and transport until a specification names a surface, and the two
surfaces now do. This ticket is that implementation, as a prototype, and the spike
decision 0014 asked for.

The maintainer's brief, given on 2026-10-31, in its own words: "Create a prototype/poc of
Pachinko based on the allium spec. Also go ahead and add a minimal but sufficient api to
start with. Use tokio. Use Apache Arrow for both the data transport for the API, but then
also for internal processing. Store the records, well, not store, but have the records
within the pocket cubes be Arrow tables. The purpose of this is for performance testing
and comparison." The maintainer named two sources as useful: Martin Kleppmann's "How to do
distributed locking" (2016), at `https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html`,
and Nathaniel J. Smith's "Notes on structured concurrency, or: Go statement considered
harmful" (2018), at `https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/`.
The agent reads both before Phase 2.

The decisions the maintainer took when the ticket was planned, each witnessed below by a
design rule, a file or an open point:

| Topic | Decision |
| --- | --- |
| Where Arrow lives | In the service crate. `arrow-array` depends on `ahash` with its `runtime-rng` feature on every non-wasm target, and `runtime-rng` turns on `getrandom`, one of the eight crates `just effects` bans from libpachinko's normal dependency tree (decision 0008). The ban stands: libpachinko keeps the pure queue over a record-store trait, with a `BTreeMap` implementation beside the trait as its reference, and the Arrow implementation lives in `crates/pachinko`, where every effect is allowed. No gate, script, fixture or ban list changes. |
| Transport | Arrow Flight over gRPC: `arrow-flight`, `tonic` and `prost`. Flight is the Arrow project's own transport, its protobuf code ships pre-generated so no `protoc` is needed, and a `pyarrow` or ADBC client can drive the service for load tests later. No TLS feature is enabled, so `deny.toml`'s `openssl-sys` ban is untouched. |
| Comparison baseline | Arrow only, for now. The `BTreeMap` records are libpachinko's test implementation of the trait, not a benchmark subject; a comparative benchmark is a later ticket. |
| Ticket shape | One ticket in phases, each ending in a commit and, where marked, an adversarial review, so one agent carries it in one worktree. |
| Runtime | tokio, as the maintainer asked. Decision 0019 supersedes 0014 and records it; this prototype is the spike's evidence. |

Two design rules come from the sources. From Kleppmann: a lease is identified by a
monotonic token the queue hands out once and never reuses, and every action under the
lease carries that token, so an action from a consumer whose lease has lapsed is rejected
rather than landing on a successor lease; the module already says actions name their
lease (`ActionsNameTheirLease`), and the token is its wire form. From the structured
concurrency essay: no task is spawned and forgotten; the service's tasks live in one
`JoinSet` that the serving function owns and joins on shutdown, and the queue has one
owner task through which every command, the clock's lapses included, is processed in one
stream, which is how the module's expiry comment says lapses and consumer actions meet.

The specification wins on *what* (decision 0002). Where the module leaves a placeholder,
the code follows the placeholder: a publish whose rank equals the stored rank is rejected,
because no rule covers it (open question 1). No open question is answered here. If a rule
proves unimplementable as written, the agent stops and hands it back as a `spec-change`
rather than deciding in code.

### Design

The agent may refine a name; it may not move the split.

#### libpachinko

The library is std, performs no effect, and names nothing from `tokio` or Arrow. Modules
under `crates/libpachinko/src/`, each a file beside a directory if it grows (never
`mod.rs`), each public type `Send + Sync + 'static`, `Clone`, `Debug` and
`#[non_exhaustive]`, each public item with an asserting doctest:

- `clock`: `Timestamp` (a value type; the agent picks the representation and documents
  it), `pub trait Clock: Send + Sync + 'static { fn now(&self) -> Timestamp; }`, and
  `FakeClock` beside it, which a test sets and advances. This is the first effect trait,
  decision 0003 applied; it needs no new decision record.
- `config`: `Config` with the module's five fields and defaults (`lease_ttl` 5 minutes,
  `max_consecutive_expiries` 3, `max_pull_batch` 100, `superseded_ttl` 12 hours,
  `dead_letter_ttl` 12 hours), and `Config::validate`, which enforces the five config
  invariants (`ExpiryLimitPositive`, `PullBatchPositive`, the three non-negative TTLs) and
  returns `ConfigError`. Each figure is mirrored once, by name, as
  `docs/reference/configuration.md` requires.
- `records`: `pub trait PocketRecords: Send + Sync + 'static + Debug + Default`, one
  instance per pocket, with an associated `type Batch: Send + Sync + Clone + Debug +
  'static`. Its operations are the record-level steps the rules take and nothing more:
  look a key up (rank, status, held rank); insert a queued record at a position
  (`PublishIntoNewPocket`, `PublishNewKey`); supersede in place, returning the old rank
  and payload (`Supersede`); hand the n oldest queued records by position as a `Batch` and
  hold them (`PullWithinLease`, `PullClaimsFrontPocket`, `HoldRecord`); settle a held key by
  removing it (`AckSettlesRecord`, `NackSettlesRecord`); defer a key at a fresh position
  (`AckDefersNewerVersion`, `NackDefersNewerVersion`); free every held and deferred record
  back to queued (`RequeueHeldRecord`, `RequeueDeferredRecord`); the record count, and the
  held records as the `Consuming` surface exposes them. `MemoryRecords`, a `BTreeMap`
  implementation whose `Batch` is a `Vec` of held records, sits beside the trait as its
  reference and the library's test store. `std::collections::HashMap` is banned by
  `clippy.toml`; every map is a `BTreeMap`.
- `queue`: `Queue<R: PocketRecords, C: Clock>`, the module's `Queue` entity and its one
  deployment: `Config`, `next_position`, pockets by key, the pocket order keyed by
  position, leases keyed by `LeaseId`, publishers, consumers, superseded records and dead
  letters. One method per trigger: `publisher_joins`, `consumer_joins`, `publish`, `pull`,
  `ack`, `nack`, `renew`, `release`, and `tick`, which processes `LeaseLapses` for every
  active lease past its `expires_at` and the two decays; plus `lease_view(consumer)` for
  the surface's `exposes`. `pull` returns a `Batch` outcome or `NoPocketAvailable`, never
  an error. `LeaseId(u64)` is taken from the queue's counter, monotonic and never reused:
  the fencing token. Ack, nack, renew and release take the `LeaseId` and reject one that is
  not active and the consumer's. The struct and method counts stay under rustqual's limits
  (`rustqual.toml`: 12 fields, 20 methods, 60-line functions, 500-line files), so the
  agent splits by rule family (publish, pull, settle, lease, expiry, decay) rather than
  suppressing.
- `error`: the error enums, through `thiserror`. This is the first error type, so this
  ticket adds `thiserror` with its decision-0007 note and the tenth invariant to
  `AGENTS.md`, "Errors are `thiserror` enums", as `CONVENTIONS.md` §7 says the change that
  adds the first error type does.

Terminology holds: the type is `Queue`, never engine; the party that pulls is the
consumer; whatever calls the library is the caller.

#### pachinko

The service depends on `libpachinko` and supplies the effects. Modules under
`crates/pachinko/src/`:

- `arrow`: `ArrowRecords`, an implementation of `PocketRecords` whose `Batch` is
  `arrow_array::RecordBatch`. Each pocket's records are one `RecordBatch` with the schema
  `record_key: Utf8`, `rank: Int64`, `payload: Binary`, `position: UInt64`,
  `status: UInt8`, `held_rank: Int64` nullable, `held_payload: Binary` nullable. Every
  operation uses the Arrow kernels (`arrow_ord` for the sort by position, `arrow_select`
  for take, filter and concat) and rebuilds the batch, because that cost is what the
  prototype measures; a hand-written row loop is used only where no kernel applies, and
  the module comment says where.
- `clock`: `SystemClock`, the real `Clock` over `std::time::SystemTime`.
- `actor`: one tokio task owns `Queue<ArrowRecords, SystemClock>`. The Flight handlers and
  a `tokio::time::interval` ticker send commands through one `mpsc` channel and receive
  replies through `oneshot` channels, so a lapse and a consumer's action are processed in
  one stream. The ticker calls `tick`. Every task is spawned into a `JoinSet` that `serve`
  owns; shutdown closes the channel and joins the set.
- `flight`: the `arrow_flight::flight_service_server::FlightService` implementation. The
  mapping, with every body and reply an Arrow IPC stream:
  - `do_put` is `Publish`. The descriptor's command names the publisher; each
    `RecordBatch` carries `pocket_key: Utf8`, `record_key: Utf8`, `rank: Int64`,
    `payload: Binary`; each batch is answered by one `PutResult` whose body is an IPC
    batch with one row per input row and its outcome: new, superseded, stale or rejected.
  - `do_get` is `Pull`. The ticket's body is a one-row IPC batch, `consumer_id: Utf8`,
    `limit: Int64`. The reply stream's schema metadata carries `outcome` (`batch` or
    `no_pocket`), `lease_id`, `pocket_key` and `expires_at`; a `no_pocket` reply is an
    empty stream, never a gRPC error, as the module's pull outcomes require.
  - `do_action` carries `publisher_joins`, `consumer_joins`, `ack`, `nack`, `renew`,
    `release` and `status`, the last returning the `Consuming` surface's `exposes` for one
    consumer. A rejected action, such as one naming a lease that has ended, answers with
    gRPC `FAILED_PRECONDITION` and the error's text; an unknown party with `NOT_FOUND`;
    a malformed body with `INVALID_ARGUMENT`.
  - `list_actions` lists the seven; every other method answers `UNIMPLEMENTED`.
- `cli`: `run(args, out)` keeps the no-argument behaviour, the version line, and adds
  `serve [--listen ADDR] [--lease-ttl DURATION] [--max-consecutive-expiries N]
  [--max-pull-batch N] [--superseded-ttl DURATION] [--dead-letter-ttl DURATION]`, parsed
  with std alone (no `clap`); the default address is `127.0.0.1:0`, `serve` writes
  `listening on <addr>` once bound, so a test may pass port 0 and read the address, and
  a bad argument or a `Config::validate` failure exits non-zero with the error on
  standard error. `pub async fn serve(config, listener)` is the in-process entry the
  integration tests call; `main.rs` stays one line of behaviour unless `run`'s signature
  forces a second.

#### Dependencies

Each is the newest release on crates.io on the day, written as a full `x.y.z` caret range
in `[workspace.dependencies]` with `workspace = true` in the crate, with its decision-0007
note in the manifest, and each lands in the phase that first uses it, because
`just deps-unused` rejects a dependency nothing uses:

- libpachinko: `thiserror` (the error enums). Development: `proptest`, the property-based
  framework `propagate` names for Rust.
- pachinko: `tokio` (features `rt-multi-thread`, `macros`, `net`, `sync`, `time`,
  `signal`), `arrow-array`, `arrow-schema`, `arrow-select`, `arrow-ord`, `arrow-ipc`,
  `arrow-flight`, `tonic`, `futures`, and only what `cargo shear` proves used beyond them
  (`bytes`, `prost` or `tokio-stream` if the Flight types force a direct use). No `tls-*`
  feature anywhere.

Adding a normal dependency to libpachinko changes `fuzz/Cargo.lock` as well as
`Cargo.lock`; both are regenerated by `just lock` (network, authorised in Phase 2) and
proved by `just lock-check`, never edited by hand. `just deny` must stay green: every
Arrow, tonic and tokio crate is Apache-2.0 or MIT, and a licence outside `deny.toml`'s
list is handed back, not added.

## Goal

`pachinko serve` starts a service that offers the two surfaces over Arrow Flight;
`libpachinko::Queue` implements every rule of `docs/specs/pachinko.allium` against a
record-store trait, through the first effect trait, the clock, with a fake beside it; the
service's Arrow record store passes the same contract and holds each pocket's records as
one `RecordBatch`; the tests `propagate` derives cover the module's obligations, with every
uncovered one classified; a divan benchmark measures the Arrow store; decision 0019
supersedes 0014 and decision 0020 records the records representation; the handbook says
what exists; `just check` is green.

## Non-goals

- No persistence, snapshot, replication or recovery: the module's Excludes.
- No authentication, authorisation or TLS on the Flight service; it listens on loopback by
  default and the security page says so.
- No capacity shedding. `NeverRefusedForCapacity` holds because nothing is ever refused;
  memory is unbounded, and open question 3 stays open.
- No answer to any of the module's nine open questions, equal rank included.
- No surface for reading the superseded or dead-letter queues (open question 2); the
  entities and their decay are implemented, with no way to read them.
- No comparative baseline benchmark, and no third crate.
- No change to `docs/specs/pachinko.allium`. A rule that cannot be implemented as written
  is handed back, not edited.
- No metrics, tracing or logging beyond the one `listening on` line.
- No change to `Justfile`, `scripts/check-effects.sh`, `tests/fixtures/effects-violation/`,
  either `clippy.toml`, `deny.toml`, `Dockerfile` or any workflow. The binary with no
  arguments keeps printing `pachinko <version>` and exiting 0, so `tests/binary.rs`'s
  first case, the first-change tutorial and CI's `image` job, which runs
  `docker run --rm pachinko:dev`, stay true without edits.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `crates/libpachinko/src/clock.rs` | new | `Timestamp`, `Clock`, `FakeClock` |
| `crates/libpachinko/src/config.rs` | new | `Config`, its defaults, `validate`, `ConfigError` |
| `crates/libpachinko/src/records.rs` and submodules | new | `PocketRecords`, `MemoryRecords`, the record and batch value types |
| `crates/libpachinko/src/queue.rs` and submodules | new | `Queue`, `LeaseId`, the outcomes, one submodule per rule family |
| `crates/libpachinko/src/error.rs` | new | The error enums |
| `crates/libpachinko/src/lib.rs` | final | Crate docs naming the module as implemented; the modules; `version()` stays |
| `crates/libpachinko/tests/*.rs` | new | The propagated tests through `MemoryRecords` and `FakeClock`; the records contract |
| `crates/libpachinko/Cargo.toml` | shared, own lines | `thiserror`, `proptest`, each with its note |
| `crates/pachinko/src/arrow.rs` and submodules | new | `ArrowRecords` and its schema |
| `crates/pachinko/src/clock.rs` | new | `SystemClock` |
| `crates/pachinko/src/actor.rs` | new | The owner task, its commands and the ticker |
| `crates/pachinko/src/flight.rs` and submodules | new | The Flight service and the IPC encodings |
| `crates/pachinko/src/cli.rs` | new | Argument parsing and `serve` |
| `crates/pachinko/src/lib.rs` | final | `run`, `serve`, the modules |
| `crates/pachinko/src/main.rs` | shared, own lines | Only if `run`'s signature changes; otherwise untouched |
| `crates/pachinko/tests/binary.rs` | final | The no-argument case kept; `serve --listen 127.0.0.1:0` started, its line read, stopped |
| `crates/pachinko/tests/flight.rs` | new | The lifecycle over loopback through `arrow_flight::FlightClient` |
| `crates/pachinko/tests/arrow_records.rs` | new | The records contract over `ArrowRecords` |
| `crates/pachinko/benches/arrow.rs` | new | The divan benchmark over `ArrowRecords` and `Queue` |
| `crates/pachinko/Cargo.toml` | shared, own lines | The service dependencies with their notes; the `[[bench]]` |
| `Cargo.toml` | shared, own lines | `[workspace.dependencies]` entries |
| `Cargo.lock`, `fuzz/Cargo.lock` | generated | By `just lock`; never hand-edited |
| `AGENTS.md` | shared, own lines | Invariant 10; the identity paragraph no longer says the domain is undefined |
| `docs/decisions/0019-service-runtime-tokio-and-flight.md` | new | Supersedes 0014: tokio, Arrow Flight, the owner task and `JoinSet`, the fencing-token lease id; the evidence is this prototype |
| `docs/decisions/0020-records-as-arrow-batches.md` | new | Records as one `RecordBatch` per pocket in the service; the `getrandom` finding; the trait split; the comparison deferred |
| `docs/decisions/0014-service-runtime-deferred.md` | shared, own lines | A superseded marker under its heading naming 0019 |
| `docs/decisions/README.md` | shared, own lines | Two rows; the supersession sentence; "The next decision is 0021" |
| `docs/manifest.yml`, `docs/README.md` | shared, own lines | The two records registered and listed |
| `docs/explanation/architecture.md` | shared, own lines | The clock trait exists; the service's owner task, ticker and Flight; "What is deferred" rewritten |
| `docs/explanation/security-model.md` | shared, own lines | The network surface: unauthenticated Flight on loopback by default |
| `docs/explanation/specifications.md`, `docs/explanation/layering.md` | shared, own lines | Specified and implemented; the Rust modules follow the one specification module |
| `docs/project/purpose-and-scope.md` | shared, own lines | "What it does"; the two non-goal bullets that said nothing is implemented and no runtime is chosen |
| `docs/project/terminology.md` | shared, own lines | Repository rows for record store, clock, lease id and Flight; the Service row no longer says no runtime is chosen; the Effect trait row no longer says none exists |
| `docs/project/repository-map.md` | shared, own lines | The `docs/specs/` row; the crate rows |
| `docs/how-to/test-and-debug.md` | shared, own lines | Running `pachinko serve` and the loopback test |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | The module section: implemented in the prototype |
| `docs/tutorials/first-change.md` | shared, own lines | The line that said the domain is undefined |
| `docs/reference/configuration.md` | shared, own lines | The dependencies; "Values the specifications decide" names the five figures and where they are mirrored |
| `docs/reference/testing.md` | shared, own lines | "Tests from the specifications" and "Benchmarks" describe what now exists |
| `docs/reference/api.md` | shared, own lines | Both crates' public API summary |
| `README.md`, `crates/libpachinko/README.md`, `crates/pachinko/README.md` | shared, own lines | What exists; how to run the service |
| `CHANGELOG.md` | shared, own lines | Both crates' Unreleased, Added |
| `tickets/README.md` | shared, own lines | The index row's status; the bootstrap paragraph |
| `tickets/T20-pachinko-prototype.md` | ticket | `status:` and hand-back notes |

Not touched, by design: `Justfile`, `scripts/check-effects.sh`,
`tests/fixtures/effects-violation/`, `clippy.toml`, `crates/libpachinko/clippy.toml`,
`deny.toml`, `Dockerfile`, `.github/workflows/*`, `docs/specs/pachinko.allium`, the
skills and bridges. A change that proves necessary in any of them is handed back.

## Steps

0. **Worktree and baseline.** Create the worktree from `main` after T19 has merged
   (`README.md`, "How to pick up a ticket"), run `just initialize` (network, authorised
   here), run `just check` on the untouched worktree and quote its last line as the
   baseline, then `just plan-spec pachinko` and quote its obligation count (190 at T19's
   `bb90641`). Read `CONVENTIONS.md` §11 and §14, the `plan-change` skill, the module, and
   the two sources named in Context.
1. **Decisions first.** Use the `review-docs` skill. Write decision 0019, which supersedes
   0014 and records tokio, Arrow Flight, the owner task with its `JoinSet`, and the
   fencing-token lease id, with this prototype as the spike's evidence and the structured
   concurrency essay and Kleppmann's post as its sources; write decision 0020, which
   records one `RecordBatch` per pocket in the service, the `getrandom` finding that put
   it there, the record-store trait split and the deferred comparison. Mark 0014
   superseded under its heading; register both in `docs/manifest.yml`, list both in
   `docs/README.md` and `docs/decisions/README.md`. Run `just check-docs`. Commit.
2. **libpachinko.** Use the `rust-change` skill, then the `propagate` skill in spec mode
   against the module and the Design above, writing the tests before the code against the
   intended API, so that they fail first; `FakeClock` is the seam the temporal tests need.
   Then implement `clock`, `config`, `records` with `MemoryRecords`, `queue` and `error`,
   adding `thiserror` and `proptest` with their notes, `just lock` (network, authorised
   here) for both lockfiles, and invariant 10 in `AGENTS.md`. Run `just fmt-check`,
   `just clippy`, `just test`, `just effects`, `just metrics`, `just coverage`, `just doc`,
   `just check-agents` and `just lock-check`, fixing each at its root. Quote propagate's
   reconciliation line, `N obligations, M covered, K uncovered`, and classify each
   uncovered obligation as the skill says. Commit.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint 1, over the public API later
   phases build on. Ask: whether every rule of the module has a method and a test that
   fails without it; whether any method can leave a pocket, record or lease in a state no
   rule leaves, or skip a `PocketTouched`, `Handed` or `Freed` chain; whether a lease id
   can ever be reused or an action land on a successor lease; whether `tick` processes a
   lapse that a renewal, ack, nack or pull processed first should have stopped; whether
   any effect is reached directly, including through a test; and whether a public type
   misses invariant 3.
3. **The Arrow store and the benchmark.** `ArrowRecords`, `SystemClock`, and the records
   contract run over `ArrowRecords` in `crates/pachinko/tests/arrow_records.rs`. The
   contract is written once; the agent either exposes it from libpachinko behind a
   `testing` feature or repeats the list, and records which and why in the hand-back
   notes. Add `crates/pachinko/benches/arrow.rs`: publish N records into M pockets, pull
   batches, ack them, with N and M as divan arguments; run `just bench` once and quote its
   output (timings measure the machine and are outside the gate). Add the Arrow crates
   with their notes. Run `just deny`, `just deps-unused`, `just features`, `just effects`
   (libpachinko's tree still names no Arrow crate) and `just coverage`. Commit.
4. **The Flight API and the service.** `actor`, `flight`, `cli`, `serve`, and the tokio,
   tonic and Flight crates with their notes. `crates/pachinko/tests/flight.rs` starts
   `serve` on `127.0.0.1:0` in-process and drives it with `arrow_flight::FlightClient`:
   both parties join; a publish into a new pocket, a new key, a supersede, a stale rank
   and an equal rank, each outcome read from the `PutResult`; a pull that claims the
   front pocket and one within the lease; ack of a held version, ack that defers a newer
   version, nack to a dead letter; renew; release; a pull that finds no pocket; an action
   naming an ended lease rejected with `FAILED_PRECONDITION`; and a lapse under a short
   `--lease-ttl`, observed through the ticker. The expiry rules themselves are proved in
   libpachinko through `FakeClock`; the loopback test proves only that the ticker reaches
   `tick`, and never sleeps against the wall clock to position itself at a deadline.
   `tests/binary.rs` keeps the no-argument case and adds `serve --listen 127.0.0.1:0`,
   reading `listening on`, then stopping the process. Run the Phase 2 recipes again plus
   `just deny` and `just deps-unused`. Commit.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint 2, over the wire mapping and
   the task model. Ask: whether every `provides`, every `exposes` line and every
   `@guarantee` of the two surfaces is reachable over Flight and tested; whether two
   consumers can be handed a record from one pocket; whether a lapse and an ack can be
   processed outside the one stream; whether a `no_pocket` reply or a non-positive limit
   ever surfaces as an error; whether shutdown can leave a task running or a client
   hanging; whether the service reads the clock or the network anywhere the library
   should have asked for it; and whether the binary with no arguments still prints its
   line and exits 0.
5. **The handbook ripple and the hand-back.** Use the `review-docs` skill over every page in
   Files touched: the sentences that said the domain is undefined, nothing is implemented
   or no runtime is chosen, including the ten places T19's Open points list; the
   terminology rows; `CHANGELOG.md`; the three READMEs; `tickets/README.md`. Run
   `just check-docs`, then `just check`.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint 3, the final one, over the
   whole diff against Goal, Files touched and the acceptance criteria. Ask: whether a file
   outside Files touched changed; whether a page now says something false; whether a gate
   passes while a claim below is untrue, in particular whether any propagated test was
   weakened to pass, whether coverage reaches 90% through tests that assert, and whether
   `just effects` would still fail on an Arrow crate in libpachinko's tree.

   Then `just check` once more, commit with the hand-back notes quoting each output headed
   by its commit, and `git status --short`. Pushing and opening the pull request are
   separately authorised: stop and ask.

## Acceptance criteria

- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing after the final commit.
- `just effects` is green, and the hand-back quotes the positive control: `cargo tree -p
  libpachinko -e normal --all-features --target all --prefix none --locked --offline`
  lists `libpachinko` itself and names no `arrow-*`, `ahash`, `getrandom` or `tokio`.
- Propagate's reconciliation line is quoted; every uncovered obligation is listed with its
  classification and reason; no generated test was edited to pass.
- `crates/pachinko/tests/flight.rs` proves, over loopback, every case Step 4 lists.
- The no-argument binary prints `pachinko <version>` and exits 0, so `tests/binary.rs`'s
  first case, the tutorial and the `image` job are unchanged.
- `just bench` output for `crates/pachinko/benches/arrow.rs` is quoted once, headed by its
  commit and naming the machine.
- Decisions 0019 and 0020 exist, are registered and listed, 0014 carries its superseded
  marker, and `just check-docs` is green.
- `AGENTS.md` carries invariant 10 and `just check-agents` prints
  `Validated AGENTS.md, 2 adapters, and 14 skills.`
- Every new dependency carries its decision-0007 note in the manifest beside its range,
  `just deny` and `just deps-unused` are green, and no `tls-*` feature is enabled.
- Each public type is `Send + Sync + 'static`, `Clone`, `Debug` and `#[non_exhaustive]`,
  each public item has an asserting doctest, and no `allow` attribute was added.
- The three adversarial reviews ran, and every valid finding is fixed or handed back.

## Verification

```sh
just plan-spec pachinko
just effects
just test
just coverage
just deny
just deps-unused
just check-docs
just check-agents
just bench
just check
git status --short
```

## Hand-back notes

### Worktree and baseline

### Decisions

### libpachinko

### Adversarial review, checkpoint 1

### The Arrow store and the benchmark

### The Flight API and the service

### Adversarial review, checkpoint 2

### The handbook

### Adversarial review, checkpoint 3

### Final gate

## Open points

- The comparison baseline is deferred by the maintainer's choice: `MemoryRecords` is the
  trait's reference and the library's test store, not a benchmark subject. A later ticket
  adds a comparative benchmark over the two stores behind the same trait.
- A publish whose rank equals the stored rank is rejected, as the module's placeholder
  says, and reported as `rejected` in the `PutResult`; open question 1 decides otherwise
  later, and the code follows.
- No authentication or TLS: the service binds loopback by default and `--listen` is the
  operator's choice. A deployment beyond one host needs a decision first.
- Memory is unbounded; superseded records and dead letters decay by TTL only. Open
  question 3 covers the rest.
- The gRPC status for a rejected action (`FAILED_PRECONDITION`), an unknown party
  (`NOT_FOUND`) and a malformed body (`INVALID_ARGUMENT`) are this ticket's choices for
  the wire; the module says nothing about status codes, and a specification change may
  move them.
- The `LeaseId` is the wire form of the lease the module identifies by entity. The module
  carries no id field; whether to add one, so that the specification names what the wire
  carries, is a `spec-change` for the maintainer.
- Whether the records contract is shared through a `testing` feature of libpachinko or
  repeated in the service's tests is the agent's choice, recorded in the hand-back notes.
- Renovate's `cargo` manager reads the new ranges automatically (decision 0018); no
  `renovate.json` change is expected, and the agent says so if one proves necessary.
- `pyarrow.flight` or an ADBC client driving the service from outside the workspace is
  the load-test tooling the transport choice makes possible; none is written here.
