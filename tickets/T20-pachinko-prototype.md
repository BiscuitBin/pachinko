---
id: T20
title: "Pachinko prototype: the queue in libpachinko, Arrow records and a Flight API in the service"
status: done
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

The raw logs, briefs, findings and verdicts quoted below are under `ai_tmp/` and
`ai_tmp/review/T20-<n>/`, not committed.

### Worktree and baseline: `c8d1e46`

The worktree was created on the branch `tickets/t20-pachinko-prototype-complete`, from
`main` at `611a86e` (the merge of T19's pull request 34), and this ticket was committed on
top as `c8d1e46`. The work is committed on that branch, not on `ticket/t20-pachinko-prototype`,
which the `branch:` field names; the field is left as planned. `just initialize` ran
(network, authorised in Step 0). Toolchain 1.99.1 and `allium` 3.6.1 were already
installed, `cargo fetch --locked` and `pixi install --frozen` ran, and `install-hooks` was
skipped because this is a secondary worktree. Both sources named in Context, Kleppmann
(2016) and Smith (2018), were read before Phase 2.

```text
just initialize
Ready. Next: just check.
Nothing has been formatted, staged, committed, tagged, or pushed.
Exit status: 0

just check
All checks passed and the worktree is unchanged.
Exit status: 0

just plan-spec pachinko   (counted with python from the saved JSON; exit status not saved)
190 obligations, 0 diagnostics
```

### Decisions: `edf9f1d`

Decision 0019 supersedes 0014. It records tokio, Arrow Flight over gRPC, the single owner
task with its `JoinSet`, and the lease id as a fencing token. Its sources are Smith's
essay and Kleppmann's post, and this prototype is the spike's evidence. Decision 0020
records one `RecordBatch` per pocket in the service. It also records the finding that put
Arrow there (`arrow-array` turns on `ahash`'s `runtime-rng`, which brings in `getrandom`,
banned from libpachinko by `just effects`), the record-store trait split and the deferred
comparison. 0014 carries a superseded marker naming 0019. Both records are registered in
`docs/manifest.yml` and listed in `docs/README.md` and `docs/decisions/README.md`, which
now says the next decision is 0021.

Later commits corrected both records in place:

- `7fe8914` and `f3a8aca` fixed the public paths (`libpachinko::queue::LeaseId`,
  `libpachinko::queue::Queue`, `libpachinko::records::PocketRecords`).
- `f1e5e68`, `2da9581` and `0aa1df4` changed 0019's shutdown paragraph.
- `3517454` changed 0019's lapse timing and 0020's effects-gate sentence.

No `just check-docs` output from `edf9f1d` was kept; the final gate covers both records.

### libpachinko: `70a333e`

The commit adds these modules:

- `clock`: `Timestamp`, `Clock` and `FakeClock`.
- `config`: `Config` and `Config::validate`.
- `records`: `PocketRecords` and `MemoryRecords`, plus the records contract behind the
  `testing` feature.
- `queue`: `Queue` and `LeaseId`, with one submodule per rule family (publish, pull,
  settle, renew and release, expiry, decay) and helper submodules for leases, pockets,
  parties, the view and the value types.
- `error`: the `thiserror` enums.

It also adds `thiserror` and the development dependency `proptest`, each with its note.
`Cargo.lock` and `fuzz/Cargo.lock` change in the same commit, and `just lock-check` passes.
`AGENTS.md` gains invariant 10.

The tests came first. The `propagate` skill ran in spec mode and wrote them against the
intended API. At `edf9f1d` with uncommitted changes, with the rules stubbed, the `queue`
target ran 95 tests. The executing agent reported 79 failing on the stubs and 16 passing
against the code written alongside them. Only the tail of that run was kept:

```text
        FAIL [   0.008s] (94/95) libpachinko::queue surfaces::publishing_provides_joining_and_publishing
        FAIL [   0.008s] (95/95) libpachinko::queue surfaces::no_other_consumer_is_handed_a_record_from_a_leased_pocket
error: test run failed
```

Each test carries a `// obligation: <id>` tag. The reconciliation was recomputed at
`0aa1df4` from `just plan-spec pachinko`'s JSON and those tags. 184 obligations have a
directly tagged test. The other six are covered by the proptest state machine in
`crates/libpachinko/tests/queue/invariants.rs`, whose header names them:

```text
190 obligations, 190 covered, 0 uncovered
```

The six are `rule-failure.PullWithinLease.2`, `rule-failure.ReleaseRequeuesPocket.3`,
`rule-failure.ReleaseIdlesEmptyPocket.3`, `rule-failure.LeaseExpiryParksPocket.3`,
`rule-failure.LeaseExpiryKeepsPocketAtFront.3` and
`rule-failure.LeaseExpiryIdlesEmptyPocket.3`. Each guard requires the lease's pocket to be
leased. `LeasedHasOneActiveLease` and `UnleasedHasNoActiveLease` make that true of every
active lease, so no direct test can make the guard fail. The machine checks both
invariants after every step.

The three TTL invariants hold by representation, because the TTLs are
`std::time::Duration` and cannot be negative. The executing agent reported two more checks.
A mutation that idles the pocket where the lapse should park it made the proptest fail. No
generated test was weakened to pass, and the one fix before green was a test-ordering bug
in the proptest harness itself.

The records contract is shared, not repeated. `libpachinko::records::contract::check_contract`
is written once, behind libpachinko's `testing` feature. `crates/libpachinko/tests/records_contract.rs`
runs it over `MemoryRecords`, and `crates/pachinko/tests/arrow_records.rs` runs it over
`ArrowRecords`, so both stores run the same list and cannot drift apart. Only two things
turn the feature on: the service's dev-dependency and libpachinko's test targets, through
`required-features`. No normal build has it.

Phase 2 gates, at `edf9f1d` with uncommitted changes (`ai_tmp/phase2-gates.log`, the
parenthesised figures from `ai_tmp/p2-*.log`):

```text
just fmt-check -> exit 0
just clippy -> exit 0
just test -> exit 0          (nextest: 99 tests run: 99 passed, 0 skipped)
just effects -> exit 0: effects: the fixture trips every ban exactly once, and pachinko trips none.
just metrics -> exit 0
just coverage -> exit 0      (TOTAL lines 97.86%)
just doc -> exit 0
just check-agents -> exit 0: Validated AGENTS.md, 2 adapters, and 14 skills.
just lock-check -> exit 0: ✔ Lock-file was already up-to-date
just toml-check -> exit 0
just deps-unused -> exit 0:   ✓ no issues found
```

### Adversarial review, checkpoint 1: `70a333e`

**Harness and model.** Codex CLI 0.165.0 with `gpt-6-astra` ran both reviewers through
subagents. Each ran read-only (`sandbox: read-only`) with a fresh context and one lens, over
`git diff main...70a333e`. Fresh Claude subagents did the triage, one per deduplicated
finding, and the fixes were applied one at a time by separate subagents. Phase 3 went on in
parallel and was committed as `81c649b` before the fixes landed as `7fe8914`.

The reviewers found six issues under the conformance lens and five under the breakage
lens, which deduplicate to seven:

- clock-regression (conformance F2, breakage F1; medium): a clock that steps back let
  renew and release stamp `expires_at` or `ended_at` before `granted_at`, against
  `GrantedBeforeExpiry` and `EndedAfterGrant`. Valid; reproduced through `FakeClock::set`.
  Applied: the queue keeps a private high-water mark, so its `now` never runs backward,
  with a regression test in `tests/queue/lease.rs`.
- nack-now (conformance F1, breakage F2; medium, triage low): a nack read the clock twice,
  so the lease's expiry and the dead letter's stamps could differ by a millisecond. Valid.
  Applied: the nack reads the clock once, with a regression test in `tests/queue/settle.rs`
  that uses a stepping clock.
- Invariant 3 on `Queue` (conformance F3, breakage F4; low): a `Clock` without `Clone` and
  `Debug` gave a `Queue` that had neither. Valid for `Queue`. Invalid for `PullOutcome`
  and `PulledBatch`: the queue only builds them over `PocketRecords::Batch`, which is
  bounded, so only a caller can build one that breaks invariant 3. Applied:
  `Clock: Clone + Debug + Send + Sync + 'static`, with a unit test.
- lease-id-exhaustion (conformance F4, breakage F5; low): the lease counter wrapped in
  release builds and could reissue a fencing token. Valid. Applied: `checked_add` under an
  `#[expect(clippy::expect_used, reason = ...)]`, with two unit tests.
- conformance F5 (low): the ten `PocketRecords` methods and `Clock::now` had no asserting
  doctest of their own. Valid; rustdoc's missing-example lint flagged exactly those eleven.
  Applied: one doctest each. The associated type `Batch` needs none, because rustdoc does
  not flag it.
- conformance F6 (low): `src/fencing.rs` was outside Files touched. Valid. Moving
  `LeaseId` into a queue submodule fails `just metrics` with an `error` → `queue` coupling
  cycle. Applied: `LeaseId` moved into a nested module of `error.rs`; its public path stays
  `libpachinko::queue::LeaseId`.
- breakage F3 (medium, triage low): proptest seeds from the OS, reads the environment and
  can write regression files. Invalid: decision 0008 exempts development dependencies, the
  queue under test runs only on `FakeClock`, and `docs/reference/testing.md` documents the
  regression files.

No valid finding was high, so one round closed the checkpoint, and nothing was handed back.
The fixes were re-run at `81c649b` with uncommitted changes (`ai_tmp/cp1-gates.log`):

```text
just fmt-check -> exit 0
just clippy -> exit 0
just test -> exit 0          (nextest: 106 tests run: 106 passed, 0 skipped)
just effects -> exit 0
just metrics -> exit 0
just coverage -> exit 0      (TOTAL lines 94.28%)
just doc -> exit 0
just check-docs -> exit 0
just check-agents -> exit 0
just deny -> exit 0
just deps-unused -> exit 0
just features -> exit 0
just lock-check -> exit 0
just toml-check -> exit 0
```

### The Arrow store and the benchmark: `81c649b`

`ArrowRecords` (`crates/pachinko/src/arrow.rs` and `arrow/schema.rs`) holds each pocket's
records as one `RecordBatch` and rebuilds it through the `arrow_ord` and `arrow_select`
kernels. `SystemClock` is in `crates/pachinko/src/clock.rs`. The shared records contract
runs over `ArrowRecords` in `crates/pachinko/tests/arrow_records.rs`. The divan benchmark,
`crates/pachinko/benches/arrow.rs`, measures three things: publishing N records into M
pockets, pulling and acking them, and the store's insert-then-hold. The commit adds
`arrow-array`, `arrow-schema`, `arrow-select` and `arrow-ord` at 60.1.0, each with its
note.

The gates ran at `70a333e` with uncommitted changes, the Phase 3 change later committed as
`81c649b`. The first pass, `ai_tmp/phase3-gates.log`, recorded failures: `just test` and
`just doc` exited 101 and `just toml-check` exited 1. The per-recipe logs written after it
show the reruns. `p3-toml-check.log` holds no exit status; `toml-check` passed at the
checkpoint-1 re-run and passes in the final gate.

```text
just fmt-check -> exit 0
just deny -> exit 0: bans ok, licenses ok, sources ok
just deps-unused -> exit 0:   ✓ no issues found
just features -> exit 0
just effects -> exit 0: effects: the fixture trips every ban exactly once, and pachinko trips none.
just coverage -> exit 0      (TOTAL lines 98.77%)
just lock-check -> exit 0
just test (rerun)            nextest: 101 tests run: 101 passed, 0 skipped
just doc (rerun)             Generated .../target/doc/libpachinko/index.html and 1 other file
```

`just bench`, run once, at `70a333e` with the Phase 3 change uncommitted. Machine: Apple M5
Pro, 18 cores, 64 GiB, macOS 26.4, arm64:

```text
arrow                              fastest       │ slowest       │ median        │ mean          │ samples │ iters
├─ publish                                       │               │               │               │         │
│  ├─ 1000 records x 10 pockets    1.875 ms      │ 2.904 ms      │ 2.01 ms       │ 2.046 ms      │ 100     │ 100
│  ├─ 10000 records x 10 pockets   36.7 ms       │ 41.82 ms      │ 37.57 ms      │ 37.76 ms      │ 100     │ 100
│  ╰─ 10000 records x 100 pockets  20.39 ms      │ 23.02 ms      │ 21.02 ms      │ 21.17 ms      │ 100     │ 100
├─ pull_and_ack                                  │               │               │               │         │
│  ├─ 1000 records x 10 pockets    1.468 ms      │ 2.232 ms      │ 1.571 ms      │ 1.594 ms      │ 100     │ 100
│  ├─ 10000 records x 10 pockets   49.23 ms      │ 52.98 ms      │ 50.34 ms      │ 50.35 ms      │ 100     │ 100
│  ╰─ 10000 records x 100 pockets  14.96 ms      │ 16.11 ms      │ 15.4 ms       │ 15.43 ms      │ 100     │ 100
╰─ store_insert_then_hold                        │               │               │               │         │
   ├─ 100                          154 µs        │ 203.9 µs      │ 158.7 µs      │ 162.1 µs      │ 100     │ 100
   ├─ 1000                         2.559 ms      │ 2.89 ms       │ 2.645 ms      │ 2.666 ms      │ 100     │ 100
   ╰─ 10000                        187.7 ms      │ 390.7 ms      │ 298.2 ms      │ 296.5 ms      │ 100     │ 100
Exit status: 0
```

The run at `0aa1df4`, after every fix, is under Final gate.

### The Flight API and the service: `e21f389`

The commit adds these modules:

- `actor`: the owner task, the ticker, `QueueHandle` and `join_tasks`. Every task lives in
  one `JoinSet` that `serve` owns, and there is no `tokio::spawn`.
- `flight`, `flight/codec.rs` and `flight/actions.rs`: the `FlightService`, the IPC codec,
  the seven actions and the status mapping.
- `cli`: std-only argument parsing.
- `lib.rs`: `run(args, out)`, `serve`, `ServeOptions` and `ServeError`.

`main.rs` changed because `run` now takes the arguments: it passes them on and writes one
`pachinko: <error>` line to standard error on failure. With no arguments it still writes
`pachinko <version>`. The new dependencies are `arrow-flight`,
`arrow-ipc`, `tonic` (no default features; `transport`, `router` and `codegen`), `tokio`
with the six features, `futures` and `bytes`, each with its note. `cargo shear` found no
direct use of `prost` or `tokio-stream`, so neither was added. `cargo tree -p pachinko -e
features` names no `tls`, `rustls`, `openssl` or `ring`.

Design choices beyond the Design, from the phase report:

- `do_put` decodes every batch before it publishes any, so a malformed upload publishes
  nothing. One command publishes one batch, and an equal rank is a `rejected` row.
- A command the owner has started finishes even if its requester has gone.
- A zero tick interval is taken as 1 ms.
- A pull with a non-positive limit answers `outcome = batch` with no rows.
- The joins and settles answer a one-column `outcome` batch.

`crates/pachinko/tests/flight.rs` and its modules under `tests/flight/` hold the Step 4
cases over loopback, each named by its test:

- Both parties join: `both_parties_join_and_a_second_join_is_a_failed_precondition`.
- New pocket, new key, supersede, stale and equal rank, read from the `PutResult` as `new`,
  `new`, `superseded`, `stale` and `rejected`: `each_publish_rule_answers_its_row`.
- The front-pocket claim and a pull within the lease:
  `a_pull_claims_the_front_pocket_and_the_next_pulls_within_the_lease`.
- Ack of the held version, ack that defers, and nack to a dead letter:
  `an_ack_of_the_held_version_settles_the_record`,
  `an_ack_defers_a_newer_version_which_the_next_lease_hands_out` and
  `a_nack_dead_letters_the_record_and_no_pull_sees_it_again`.
- Renew and release: `a_renewal_moves_the_expiry` and
  `a_release_ends_the_lease_and_requeues_the_pocket`.
- No pocket: `a_pull_that_finds_no_pocket_is_a_schema_only_reply_not_an_error`.
- An ended lease rejected: `an_action_naming_an_ended_lease_is_a_failed_precondition`.
- A lapse observed through the ticker: `the_ticker_processes_a_lapse`, with a zero lease
  TTL and a 10 ms tick, polling `status` under a deadline.

`tests/binary.rs` keeps the no-argument case. It adds `serve --listen 127.0.0.1:0`, which
reads `listening on` and then kills the child, and a bad argument, which exits 1 with the
error on standard error. The commit had 24 loopback tests; review fixes brought it to 28.

The phase report's recipes ran at `7fe8914` with uncommitted changes, after
`cargo fmt --all` (`ai_tmp/p4-*.log`):

```text
just fmt-check -> exit 0
just clippy -> exit 0
just test -> exit 0          (nextest: 181 tests run: 181 passed, 0 skipped)
just metrics -> exit 0: ::notice::Quality score: 100.0% (264 functions analyzed)
just coverage -> exit 0      (TOTAL lines 95.89%)
just doc -> exit 0
just deny -> exit 0: bans ok, licenses ok, sources ok
just deps-unused -> exit 0: ✓ no issues found
just features -> exit 0
just effects -> exit 0
just lock-check -> exit 0
just toml-check -> exit 0
just check-docs -> exit 0: Validated 46 pages and 47 canonical topics.
cargo run -p pachinko --locked -> exit 0: pachinko 0.1.0
```

### Adversarial review, checkpoint 2: `e21f389`

**Harness and model.** As at checkpoint 1: Codex CLI 0.165.0 with `gpt-6-astra` through
subagents, read-only, triaged by fresh Claude subagents. Each fix was applied by one writer
at a time.

**Round 1** covered the diff through `e21f389`. The conformance run was cut off before it
wrote any findings (its log ends with no exit line), so only the breakage lens reported:

- F1 (high, triage medium): an upload left open kept `do_put` in `try_collect`, so `serve`
  never returned. Valid; reproduced in a scratch test. Decision 0019 orders the shutdown
  correctly, but tonic's graceful shutdown has no deadline. Applied: a `watch` signal
  answers the open upload `UNAVAILABLE`, which still publishes nothing. Test:
  `shutdown_answers_an_upload_still_open_unavailable_and_returns_ok`.
- F2 (high): a pocket's `Binary` and `Utf8` columns have i32 offsets, so about 2 GiB of
  payload in one pocket panicked `concat_batches` and killed the owner task. That breaks
  `NeverRefusedForCapacity`. Valid; reproduced with two 1 GiB inserts. Applied: the store's
  and the pulled batch's variable-length columns are now `LargeUtf8` and `LargeBinary`. A
  gated test pins the column types, and an `#[ignore]`d test inserts more than 2 GiB. This
  changes the `do_get` and `status` reply schemas on the wire and departs from the Design's
  `payload: Binary`; the `do_put` input schema is unchanged.
- F3 (medium, triage low): `read_one_row` ignored whatever followed the first IPC batch.
  Valid. Applied: a second batch or trailing bytes is `INVALID_ARGUMENT`, with a unit test.

The fixes landed as `f1e5e68`. Because F2 was valid at high, a second round ran.

**Round 2** reran conformance over the full scope and breakage over the fix diff:

- conformance-2 F1 and breakage-2 F1 (high): a `DoAction` or `DoGet` whose message or
  end-of-stream never arrives still blocked shutdown, because tonic awaits the unary
  message before it calls the handler. Valid; reproduced. Applied: an `UntilStopping`
  service wrapper answers a request whose handler has not begun `UNAVAILABLE`, and lets a
  begun one finish. Two regression tests, one each for `DoAction` and `DoGet`.
- breakage-2 F2 (medium, triage low): a body with its terminator replaced by one to three
  stray bytes was still accepted. Valid. Applied: a one-to-three-byte remainder is
  `INVALID_ARGUMENT`, with cases added to the codec test.
- breakage-2 F3 (low): the over-2 GiB test is excluded from the gate. Invalid: the gated
  schema test pins the root cause, the ignored test allocates several GiB, and it passes
  when run by hand (below).

The round-2 fixes landed as `2da9581`. Round 2 had a valid high finding, but §14 allows two
rounds at most, so what remains is in Open points: a `do_get` reader that stalls through
HTTP/2 flow control, and the ignored test. Nothing was handed back as out of scope. The
`2da9581` fix was not reviewed as a third round, but it falls inside checkpoint 3's diff,
`main...f3a8aca`, which reviewed it.

The round-1 re-run ran at `e21f389` with uncommitted changes. The per-recipe output is in
`ai_tmp/review/T20-2/gate-*.log`, which holds no exit lines; the triage subagent's gate
report in `verdicts.md` records all 13 recipes exiting 0: `fmt-check`, `clippy`, `test` (184 run, 184 passed, 1 skipped),
`metrics`, `coverage` (TOTAL lines 95.76%), `doc`, `deny`, `deps-unused`, `features`,
`effects`, `lock-check`, `toml-check` and `check-docs`. The round-2 re-run, at `f1e5e68`
with uncommitted changes (`gate2-*.log`; exit statuses from the gate report in
`verdicts-2.md`), exited 0 on the same 13, with `test` at 188 run,
188 passed, 1 skipped, and coverage at TOTAL lines 95.82%.

The ignored test, run at `0aa1df4`:

```text
cargo test -p pachinko --lib --locked --offline -- --ignored a_pocket_holds_more_than_2_gib_of_payload
test arrow::tests::a_pocket_holds_more_than_2_gib_of_payload ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 57 filtered out; finished in 0.53s
Exit status: 0
```

### The handbook: `f3a8aca`

The `review-docs` ripple changed these files, in Files touched: `AGENTS.md` (the identity
paragraph), `CHANGELOG.md`, `README.md`, both crate READMEs, `crates/pachinko/Cargo.toml`
(the package `description`), `crates/pachinko/src/lib.rs` (crate docs), `docs/README.md`, decisions 0019 and
0020 (public paths), `docs/explanation/architecture.md`, `layering.md`,
`security-model.md` and `specifications.md`, `docs/how-to/test-and-debug.md` and
`work-with-the-specs.md`, `docs/project/purpose-and-scope.md`, `repository-map.md` and
`terminology.md`, `docs/reference/api.md`, `configuration.md` and `testing.md`,
`docs/tutorials/first-change.md` and `tickets/README.md` (the bootstrap paragraph).

The user approved five files outside Files touched, for minimal fixes of now-false
sentences only: `SECURITY.md`, `docs/explanation/quality-philosophy.md`,
`docs/how-to/maintain-dependencies.md`, `docs/how-to/build-the-container-image.md` and
`docs/reference/quality-gates.md`. In `quality-philosophy.md` the approval named the
sentence at about line 85 ("No effect trait exists yet"). The commit also removed the
small-denominator sentence at `main`'s lines 77-79. That sentence was false too, but it
was outside the approved line, so the maintainer confirms or reverts it. Other now-false
sentences outside these files are handed back in Open points.

`just check`, at `f3a8aca` (`ai_tmp/review/T20-3/just-check.log`):

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

### Adversarial review, checkpoint 3: `f3a8aca`

**Harness and model.** As before: Codex CLI 0.165.0 with `gpt-6-astra` through subagents,
read-only, over `git diff main...f3a8aca`. A fresh Claude subagent deduplicated the
findings, and one fresh Claude subagent verified each group. Fixes were applied one at a
time.

Findings, four under the conformance lens and two under the breakage lens:

- C1 (medium): `PullOutcome<B>` and `PulledBatch<B>` leave `B` unbounded. Invalid: this
  repeats checkpoint 1's settled verdict with no new evidence. The queue only produces them
  over the bounded `PocketRecords::Batch`.
- C2 (low): decision 0020 said `just effects` fails on any Arrow crate in libpachinko's
  tree. Valid; checked against the lockfile. Applied: the sentence now names `arrow-array`
  and the crates built on it.
- C3 and B1 (low and medium, triage low): "a lapse is processed up to one tick after
  expiry" is not a bound, because a `Tick` queues behind commands already sent. Valid.
  Applied: the wording in `docs/explanation/architecture.md`, decision 0019 and
  `ServeOptions`' rustdoc; the code is unchanged.
- C4 (low): `PocketRecords::Batch` has no doctest. Invalid: checkpoint 1 settled it, and
  rustdoc does not flag an associated type.
- B2 (low): the Flight renew test asserted a strictly later expiry after a 5 ms sleep,
  which a wall-clock step back would break. Valid. Applied: the test renews until the
  expiry moves, under the scenario deadline.

The fixes landed as `3517454`. No valid finding was high, so one round closed the
checkpoint, and nothing was handed back.

The checkpoint's questions, answered:

- Files outside Files touched: only the five approved ones above, plus test modules under
  `crates/libpachinko/tests/queue/` and `crates/pachinko/tests/flight/`, which the table's
  rows name only as `tests/*.rs` and `tests/flight.rs`.
- A page that says something false: decision 0020 (C2), and the lapse timing (C3); both
  fixed. Pages outside the approved files are in Open points.
- Propagated tests weakened: neither reviewer found one, and the executing agent reports
  none.
- Coverage reached through tests that do not assert: neither reviewer found any.
- Would `just effects` still fail on an Arrow crate in libpachinko's tree: only on
  `arrow-array`, or a crate built on it such as `arrow-ord`, `arrow-select`, `arrow-ipc`
  or `arrow-flight`, because `arrow-array` brings in `getrandom`. `arrow-schema`,
  `arrow-buffer` and `arrow-data` name no banned crate and would pass. The gate bans
  effects, not Arrow.

`just check`, at `f3a8aca` with uncommitted changes, after the fixes
(`ai_tmp/review/T20-3/just-check-final.log`):

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

### Code review after checkpoint 3: `3517454`

This pass was not a §14 checkpoint. Claude Code's `/code-review` ran at high effort over
the branch, in a forked subagent of the executing session, on `claude-opus-5-5`, and
fresh Claude subagents triaged each finding. It made eight findings:

- `Stopped` was a plain struct, against invariant 10. Valid, low. Applied: a one-variant
  `thiserror` enum.
- `status` sent every held record in one `arrow_flight::Result`, which a client's 4 MiB
  receive limit rejects. Valid, low; reproduced. Applied: the reply is split across
  several messages, tested by `status_answers_when_the_held_records_exceed_one_grpc_message`.
- An owner or ticker panic went unnoticed until shutdown. Valid, low; reproduced. Applied:
  a task that ends early starts shutdown, and `serve` returns `ServeError::Task`.
- `supersede`, `defer` and `settle` in `arrow.rs` scanned the key column twice. Valid, low.
  Applied: one key mask per step. The benchmark calls neither `supersede` nor `defer`.
- Duplicated ack and nack handlers and codec reads. Invalid: a simplification with no cost.
- `do_put` buffers a whole upload in memory. Invalid as a defect: it is deliberate (it
  keeps the upload all-or-nothing), and any cap answers open question 3. In Open points.
- A later batch's failure could drop an earlier `PutResult`. Invalid: unreachable, because
  decoding precedes publishing and the only remaining error fails the first batch.
- SIGTERM is not handled, so as PID 1 `docker stop` ends in SIGKILL. Invalid against the
  stated contracts, which name only an interrupt. In Open points for the maintainer.

The fixes landed as `0aa1df4`. `just check` ran at `3517454` with uncommitted changes
(`ai_tmp/review/code-review/just-check.log`):

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

### Final gate: `0aa1df4`

The Verification block and the acceptance extras ran on the clean tree at `0aa1df4`
(`git status --short` printed nothing before and after the run), before this section was
written. The logs are under `ai_tmp/handback/`.

```text
just plan-spec pachinko   (counted with python from the JSON)
190 obligations, 0 diagnostics
Exit status: 0

just effects
effects: libpachinko names none of 8 banned crates and uses none of 121 banned std APIs;
effects: the fixture trips every ban exactly once, and pachinko trips none.
Exit status: 0

cargo tree -p libpachinko -e normal --all-features --target all --prefix none --locked --offline
libpachinko v0.1.0 (.../crates/libpachinko)
thiserror v2.0.21
thiserror-impl v2.0.21 (proc-macro)
proc-macro2 v1.0.107
unicode-ident v1.0.27
quote v1.0.47
proc-macro2 v1.0.107 (*)
syn v3.0.9
proc-macro2 v1.0.107 (*)
quote v1.0.47 (*)
unicode-ident v1.0.27
Exit status: 0

grep -nE "^libpachinko " (over that output; the positive control)
2:libpachinko v0.1.0 (.../crates/libpachinko)
Exit status: 0

grep -nE "arrow-|ahash|getrandom|tokio" (over that output)
Exit status: 1

just test
     Summary [   0.483s] 191 tests run: 191 passed, 1 skipped
all doctests ran in 1.16s; merged doctests compilation took 0.78s
Exit status: 0

just coverage   (columns: regions, missed, cover; functions, missed, executed; lines, missed, cover)
TOTAL                                      5847               242    95.86%         582                30    94.85%        3404               130    96.18%           0                 0         -
Exit status: 0

just deny
bans ok, licenses ok, sources ok
Exit status: 0

just deps-unused
  ✓ no issues found
Exit status: 0

just check-docs
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
Validated 46 pages and 47 canonical topics.
Exit status: 0

just check-agents
Validated AGENTS.md, 2 adapters, and 14 skills.
Exit status: 0

cargo run -q -p pachinko --locked
pachinko 0.1.0
Exit status: 0

just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

The skipped test is the `#[ignore]`d over-2 GiB test, quoted under checkpoint 2.
`just bench` at `0aa1df4`, on the same machine (Apple M5 Pro, 18 cores, 64 GiB, macOS 26.4,
arm64, confirmed with `sysctl` and `sw_vers`):

```text
arrow                              fastest       │ slowest       │ median        │ mean          │ samples │ iters
├─ publish                                       │               │               │               │         │
│  ├─ 1000 records x 10 pockets    1.884 ms      │ 3.122 ms      │ 2.078 ms      │ 2.101 ms      │ 100     │ 100
│  ├─ 10000 records x 10 pockets   37.82 ms      │ 45.31 ms      │ 40.23 ms      │ 39.95 ms      │ 100     │ 100
│  ╰─ 10000 records x 100 pockets  20.15 ms      │ 27.57 ms      │ 20.82 ms      │ 21.35 ms      │ 100     │ 100
├─ pull_and_ack                                  │               │               │               │         │
│  ├─ 1000 records x 10 pockets    1.308 ms      │ 1.977 ms      │ 1.37 ms       │ 1.423 ms      │ 100     │ 100
│  ├─ 10000 records x 10 pockets   43.31 ms      │ 50.78 ms      │ 44.51 ms      │ 44.89 ms      │ 100     │ 100
│  ╰─ 10000 records x 100 pockets  13.22 ms      │ 16.64 ms      │ 13.54 ms      │ 13.62 ms      │ 100     │ 100
╰─ store_insert_then_hold                        │               │               │               │         │
   ├─ 100                          158.2 µs      │ 192.9 µs      │ 162.1 µs      │ 163.4 µs      │ 100     │ 100
   ├─ 1000                         2.663 ms      │ 3.205 ms      │ 2.754 ms      │ 2.864 ms      │ 100     │ 100
   ╰─ 10000                        209.3 ms      │ 348.8 ms      │ 239.9 ms      │ 248.1 ms      │ 100     │ 100
Exit status: 0
```

Two acceptance criteria checked at `0aa1df4` (`ai_tmp/handback/allow-tls.log`):

```text
rg -c "#\[expect" crates/   (the positive control)
crates/libpachinko/src/queue/leases.rs:1
crates/pachinko/src/arrow.rs:1
Exit status: 0

rg -n "#\[allow|#!\[allow" crates/
Exit status: 1

cargo tree -p pachinko -e features --target all --locked --offline
(1153 lines)
Exit status: 0

grep -inE "tls|rustls|openssl|\bring\b" (over that output)
Exit status: 1
```

`status:` is set to `done` here and in `tickets/README.md`, and those two edits are the
only change after the runs above. `just check-docs` ran over them, at `0aa1df4` with
uncommitted changes:

```text
just check-docs
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
bg-validate-docs
Validated 46 pages and 47 canonical topics.
Exit status: 0
```

`just check` and `git status --short` at the commit that carries this section remain to be
run.

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
- SIGTERM is not handled: `serve` stops on an interrupt only. As PID 1 in the distroless
  container, the kernel ignores SIGTERM, so `docker stop` ends in SIGKILL after the grace
  period and skips the graceful join. The fix belongs in `crates/pachinko/src/lib.rs`, and
  the maintainer decides whether to make it.
- `do_put` holds a whole upload in memory before it publishes any of it, which is what
  keeps an upload all-or-nothing. A cap on batches or bytes per upload is a capacity limit
  and waits for open question 3.
- A client that stops reading a large `do_get` reply can still stall graceful shutdown
  through HTTP/2 flow control. Bounding that needs a forced-close deadline, which tonic
  offers only through `max_connection_age_grace`, and that bound applies for a
  connection's whole life.
- The over-2 GiB regression test in `crates/pachinko/src/arrow.rs`,
  `a_pocket_holds_more_than_2_gib_of_payload`, is `#[ignore]`d because it allocates more
  than 2 GiB. The gate runs the schema-type test `the_variable_length_columns_have_64_bit_offsets`
  instead. The ignored test passed when run by hand at `0aa1df4`.
- The store's `record_key`, `payload` and `held_payload` columns, and the pulled batch's
  `record_key` and `payload`, are `LargeUtf8` and `LargeBinary`, not the Design's `Utf8`
  and `Binary`. Checkpoint 2 made this change to fix the offset overflow. The `do_get` and
  `status` replies carry the Large types on the wire; the `do_put` input schema is
  unchanged.
- Coverage of `crates/libpachinko/src/queue/leases.rs` reads low: 59.54% of lines (70 of
  173 missed), down from 100% before the checkpoint-1 fix added the lease-counter
  exhaustion check and its `#[should_panic]` unit test. The executing agent attributes the
  drop to that test's lost counters; it is not established here. The queue tests exercise
  the file, and the total is 96.18%.
- The following sentences outside Files touched are now false. They are handed back
  because the scope guard keeps these files out of this change:
  - `docs/reference/agent-contract.md:40` says "the nine invariants"; there are ten.
  - `.agents/skills/review-change/SKILL.md:9` says "the nine invariants", and
    `.agents/skills/spec-change/SKILL.md:10` says the module is a placeholder that declares
    no behaviour. The skills are frozen.
  - `docs/how-to/develop-locally.md:119-120` says "the service's integration test runs the
    built binary", but the service now has three integration tests (`binary.rs`,
    `flight.rs` and `arrow_records.rs`).
  - `tickets/CONVENTIONS.md`, at lines 29, 77-79, 92-93, 118, 287, 300-303, 502, 536, 593
    and 839, describes the bootstrap state: a std binary, the runtime deferred to 0014, a
    placeholder module, no runtime dependency, `run(out)`, nine invariants and the small
    denominator. These may be read as history.
  - The `[architecture]` comment in `rustqual.toml` says there are no modules to bound yet.
  - The Excludes in `docs/specs/pachinko.allium` (line 21) still name decision 0014, now
    superseded by 0019; the specification is frozen for this ticket.
