---
id: T20
title: "Pachinko prototype: the domain in libpachinko, a tokio service with a REST API"
status: open
depends_on: [T19]
parallel_with: []
branch: ticket/t20-prototype-and-rest-api
estimated_size: L
---

# T20: Pachinko prototype: the domain in libpachinko, a tokio service with a REST API

## Context

T19 wrote the domain into `docs/specs/pachinko.allium`: a single-deployment queue of
pockets, one record per key in a pocket, a higher rank superseding in place, exclusive
renewable leases, parking after consecutive lease expiries, and decaying superseded records
and dead letters. It is specified and not implemented: `crates/libpachinko/src/lib.rs`
still holds one item, `version()`, and `crates/pachinko` writes one line. Nine open
questions stand in the module and none is answered here.

This ticket builds the prototype the maintainer asked for on 2026-10-31: the whole module
implemented in `libpachinko` behind the effect traits it needs, and a simple REST API on
the `pachinko` service, on tokio, so that a publisher and a consumer can drive a running
deployment over HTTP. It is a proof of concept of the specified behaviour, in memory, and
it is held to the full gate like any other change; what makes it a prototype is its
non-goals, not its quality.

Two things fall out of the invariants and decisions before any design choice is made:

- **The runtime decision.** Decision 0014 deferred the async runtime, transport and HTTP
  stack until "the first specification that names a surface the service exposes". T19's
  `Publishing` and `Consuming` surfaces are that trigger, so this ticket runs the spike
  0014 names and writes decision 0019, which supersedes it. The maintainer has chosen tokio;
  the spike records why, against the alternatives 0014 lists, and chooses the HTTP stack.
- **The effects.** The module reads the clock (`now`, every `expires_at`) and the service
  needs a lease identity a client cannot confuse with another's. The clock is a trait in
  `libpachinko` with a fake beside it, and its real implementation lives in `pachinko`
  (decisions 0003 and 0008). The lease-id source is a second trait of the same shape, so
  that randomness stays out of the library. `just effects` bans `tokio` and every
  `std::time::*::now` from libpachinko's tree and source, so the library is synchronous and
  pure: the service owns the runtime, the sweep that fires `LeaseLapses`, and the clock.

Two readings shaped the design below, and decision 0019 cites both:

- `https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html`. A lease is
  a fencing token. The service mints an opaque lease id on every claim, a batch names it,
  and every ack, nack, renew and release carries it; an action naming an ended lease is
  rejected rather than landing on a successor lease over the same pocket, which is the
  module's `ActionsNameTheirLease`. Expiry is judged by the service's clock alone; the
  consumer treats its lease as advisory and the prototype promises no mutual exclusion
  across processes, because storage and replication are out of the module's scope.
- `https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/`.
  No detached task. The service's `run` owns every task it starts, the listener, the lapse
  sweep and the shutdown signal, in one scope: it cancels the others when one ends or
  fails, joins them all before it returns, and returns the first error. A `tokio::spawn`
  whose handle nobody awaits is a defect in review.

The behaviour is the module's and nothing here changes it. A rule the implementation cannot
honour is a `spec-change` hand-back, never a judgement taken in code (decision 0002). The
wire shape, by contrast, is the module's Excludes ("Transport, wire format and client
authentication: the spike decision 0014 names"), so it is decided here, in decision 0019
and the handbook page that owns the route table, and it is a prototype's choice the
maintainer can replace.

### Design

Embedded here so that the executing agent does not have to guess. Where a choice is a
prototype's and the maintainer may want another, Open points says so.

#### Dependencies

Every crate below enters through `[workspace.dependencies]` in the root `Cargo.toml` with
its decision-0007 note beside the range, and each crate manifest names it with
`workspace = true` (decision 0007). The versions are those `cargo search` returned on
2026-10-31; the agent re-reads them on the day with `cargo search <crate> --limit 1`
(network, authorised by this ticket) and writes the full `x.y.z` the search returns.

| Crate | Where | Why the crate that takes it cannot do without it |
| --- | --- | --- |
| `thiserror` 2.0.21 | libpachinko, normal | The first error type. `CONVENTIONS.md` §7: the invariant "errors are thiserror enums" arrives with it, and this ticket adds it to `AGENTS.md` as invariant 10. |
| `proptest` 1.12.0 | libpachinko, dev | The property-based walk over the transition graph the `propagate` skill derives for the module's invariants; it needs a shrinking generator the standard library has no equivalent of. |
| `tokio` 1.54.1 | pachinko, normal | The async runtime decision 0019 chooses; features `rt-multi-thread`, `macros`, `net`, `signal`, `sync`, `time`. |
| `axum` 0.8.9 | pachinko, normal | The HTTP router over tokio's listener; routing, extractors and JSON responses that the service would otherwise hand-write over hyper. |
| `serde` 1.0.229 (`derive`) and `serde_json` 1.0.154 | pachinko, normal | The request and response bodies; the library's types stay serde-free, and the service owns the wire shape (decision 0019). |
| `base64` 0.22.x | pachinko, normal | Payloads are bytes and JSON has no byte type; the wire carries them base64-encoded. |
| `rand` 0.9.x or `getrandom` 0.3.x | pachinko, normal | The lease-id source: an unguessable 128-bit token is randomness, an effect the service performs (decision 0008). |
| `tracing` 0.1.44 and `tracing-subscriber` 0.3.23 | pachinko, normal | The service's log line per request and per sweep; `print_stderr` is a warning the lint table turns into an error, so the service cannot `eprintln!`. |
| `ureq` 3.5.0 | pachinko, dev | A blocking HTTP client for `crates/pachinko/tests/http.rs`, so the test drives the server as a client would without a second runtime in the test. |

Not taken: `clap` (two flags are parsed by hand), `tokio-util` (shutdown goes through
`tokio::sync::watch`), `uuid` (a random token from `rand` is enough), `tower-http`. A crate
the agent finds it needs beyond this table is added with its note and listed in the
hand-back; one that proves unnecessary is dropped, since `just deps-unused` rejects it.

After the manifests change, `just lock` (network, authorised) regenerates `Cargo.lock` and
`fuzz/Cargo.lock`; the fuzz crate names `libpachinko` by path and its lock must carry
thiserror. `just lock-check`, `just deny` and `just deps-unused` are then green before any
code is written. `just effects` must still show `tokio` absent from libpachinko's tree and
present in pachinko's; the fixture under `tests/fixtures/effects-violation/` is untouched.

#### The library

One Rust module per concern under `crates/libpachinko/src/`, each with its unit tests in
a `#[cfg(test)]` module (decision 0009), and a doctest on every public item. Every public
type is `Send + Sync + 'static`, `Clone` and `Debug`, and every public struct and enum is
`#[non_exhaustive]` (invariant 3). `HashMap` and `HashSet` are disallowed; the collections
are `BTreeMap` and `BTreeSet`, which also gives the position orders for free.

- `clock.rs`: `trait Clock: Send + Sync { fn now(&self) -> Timestamp; }`, with `Timestamp`
  as a newtype over a non-negative count of milliseconds since an epoch the service
  defines, `Timestamp + Duration` and ordering, and `FakeClock` beside it with `set` and
  `advance`. `std::time::Duration` is used as the duration type: the type is not banned,
  only `Instant::now` and the other reads are.
- `lease_ids.rs`: `trait LeaseIds: Send + Sync { fn next(&self) -> LeaseId; }` and
  `CountingLeaseIds`, the fake, which hands out 1, 2, 3. `LeaseId` is an opaque newtype
  over `u128` with a `Display` the wire can carry.
- `ids.rs`: `PublisherId`, `ConsumerId`, `PocketKey`, `RecordKey` as newtypes over
  `String`; `Rank` over `i64`; `Payload` over `Vec<u8>`; `Position` over `u64`.
- `config.rs`: `Config { lease_ttl, max_consecutive_expiries, max_pull_batch,
  superseded_ttl, dead_letter_ttl }` with the module's defaults, and a constructor that
  rejects what the five config invariants exclude (`ExpiryLimitPositive`,
  `PullBatchPositive`, the three non-negative TTLs), as T19's hand-back says the
  implementation reads them: checks at config load.
- `error.rs`: one thiserror enum, `Error`, with one variant per `requires` a surface action
  can fail: `PublisherExists`, `ConsumerExists`, `UnknownPublisher`, `UnknownConsumer`,
  `EqualRank` (the placeholder for open question 1, and its doc says so), `LeaseNotActive`,
  `LeaseNotYours`, `RecordNotHeld`, `InvalidConfig`. A pull never fails: `PullOutcome` has
  `NoPocketAvailable`.
- `queue.rs`: the aggregate `Queue<C: Clock, L: LeaseIds>` holding `next_position`, the
  publishers, the consumers, the pockets by key, the pocket order (queued pockets by
  position), the leases by id, the superseded records and the dead letters. Its
  operations, each named after the rule group it implements and documented with the rule
  names it covers:
  - `publisher_joins`, `consumer_joins`;
  - `publish(publisher, pocket_key, record_key, rank, payload) -> Result<PublishOutcome>`
    with `Created`, `NewKey`, `Superseded`, `Stale`, covering `PublishIntoNewPocket`,
    `PublishNewKey`, `Supersede`, `DropStalePublish`, `ReviveIdlePocket` and
    `ReviveParkedPocket`;
  - `pull(consumer, limit) -> Result<PullOutcome>` with `Batch { lease_id, pocket_key,
    expires_at, records }` and `NoPocketAvailable`, covering `PullWithinLease`,
    `PullClaimsFrontPocket`, `HoldRecord` and `PullFindsNoPocket`; a limit that is not
    positive yields an empty batch;
  - `ack`, `nack` (the settle and defer rules), `renew`, `release` (the requeue and idle
    rules, with `RequeueHeldRecord` and `RequeueDeferredRecord`);
  - `lapse() -> Vec<Lapsed>`: every active lease whose `expires_at <= now` is processed by
    `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront` or
    `LeaseExpiryIdlesEmptyPocket`, in lease order, and the result says which;
  - `decay() -> Decayed`: `SupersededRecordDecays` and `DeadLetterDecays`;
  - the `Consuming` surface's `exposes`: `lease_of(consumer)` returning the pocket key,
    `expires_at` and the held records with their held rank and payload, or nothing.
  Every operation takes `now` from the `Clock` the queue was built with, never as an
  argument, so the service and the tests drive time the same way. `lapse` and `decay` are
  the only rules the clock triggers; nothing inside the library calls them on its own.
  Positions come from `next_position` exactly as the module assigns them, so the four
  position invariants hold by construction and the tests check them anyway.

The ordering question open question 8 leaves to processing order is resolved in the
prototype by the mutex: whatever the service processes first wins, and a late renew that
beats the sweep extends the lease. The hand-back records this reading; it decides nothing.

#### The tests

Use the `propagate` skill in spec mode over `just plan-spec pachinko` (190 obligations at
T19's last run; the count on the day is quoted). Write the tests before the code and show
them failing, then implement rule group by rule group in the module's order. Where each
kind lives:

- Unit tests in the module they test: the clock arithmetic, the config checks, the id
  types, the error display.
- Integration tests under `crates/libpachinko/tests/`, driving the public API through
  `FakeClock` and `CountingLeaseIds`, one file per rule group: `publish.rs`, `pull.rs`,
  `settle.rs` (ack and nack), `lease.rs` (renew, release, stale lease rejected), `expiry.rs`
  (the three lapse rules, parking after `max_consecutive_expiries`, revival), `decay.rs`,
  and `invariants.rs`, the proptest state-machine walk: random valid action sequences
  against the queue, every module invariant asserted after every step, the
  `OneVisitPerKeyPerLease`, `OldestFirst` and `ExclusivePocket` guarantees asserted where
  they apply. Replay the trace T19's pull request description carries as one scenario.
- A doctest on every public item that asserts.

Temporal obligations position the fake clock before, at and after each `expires_at`; no
test sleeps. The reconciliation line, `N obligations, M covered, K uncovered`, is quoted
in the hand-back with each uncovered obligation classified as the skill requires.

#### The service

`crates/pachinko/src/lib.rs` keeps `run`, now `run(args, out)`, and `main.rs` stays one
line of behaviour (`CONVENTIONS.md` §3): it passes `std::env::args().skip(1)` and locked
standard output. With no argument, `run` writes `pachinko <version>` exactly as today, so
`tests/binary.rs` and CI's `docker run --rm pachinko:dev` are unchanged. With
`serve [--listen <addr>]` (default `127.0.0.1:8080`; `--lease-ttl-ms`, `--sweep-ms` and the
other config values as flags with the module's defaults) it:

1. builds a multi-thread tokio runtime and `block_on`s one async function, `serve`;
2. builds `SystemClock` (over `std::time::SystemTime`) and `RandomLeaseIds` (over `rand`),
   the real implementations of the two traits, and the `Queue` behind an
   `Arc<std::sync::Mutex<_>>`: every operation is synchronous and short, so a std mutex is
   taken and released inside one handler without an `.await` in between;
3. binds the listener (`127.0.0.1:0` in tests, so the test reads the bound port), builds
   the axum router below with the shared state, and starts the lapse sweep, a task that
   every `--sweep-ms` calls `lapse` then `decay` and logs what each did;
4. waits on a shutdown signal (ctrl-c, and SIGTERM on unix) and a `tokio::sync::watch`
   channel the test can also close;
5. structured as the second reading asks: the listener, the sweep and the signal are
   started in one function, joined through a `JoinSet` or a `select!` over their handles,
   every one of them ends when any ends, with the server's graceful shutdown given a
   bounded grace period, and `serve` returns only after all have been joined, with the
   first error if there was one. No `tokio::spawn` outside that function.

The routes, each the surface action it names, with JSON bodies and base64 payloads. The
table below is the page `docs/reference/http-api.md` owns; the ticket repeats it so that
the agent has it before the page exists.

| Method and path | Body | Surface action | Replies |
| --- | --- | --- | --- |
| `POST /publishers/{publisher_id}` | none | `PublisherJoins` | 201; 409 `publisher_exists` |
| `POST /consumers/{consumer_id}` | none | `ConsumerJoins` | 201; 409 `consumer_exists` |
| `PUT /pockets/{pocket_key}/records/{record_key}` | `{"publisher_id", "rank", "payload"}` | `Publish` | 200 `{"outcome": "created" or "new_key" or "superseded" or "stale"}`; 404 `unknown_publisher`; 409 `equal_rank` with a message naming open question 1 |
| `POST /consumers/{consumer_id}/pull` | `{"limit"}` | `Pull` | 200 `{"lease_id", "pocket_key", "expires_at", "records": [{"record_key", "rank", "payload"}]}` or 200 `{"no_pocket_available": true}`; 404 `unknown_consumer` |
| `POST /leases/{lease_id}/ack` | `{"consumer_id", "record_key"}` | `Ack` | 200; 409 `lease_not_active`; 403 `lease_not_yours`; 404 `record_not_held` |
| `POST /leases/{lease_id}/nack` | `{"consumer_id", "record_key"}` | `Nack` | as ack |
| `POST /leases/{lease_id}/renew` | `{"consumer_id"}` | `Renew` | 200 `{"expires_at"}`; 409; 403 |
| `POST /leases/{lease_id}/release` | `{"consumer_id"}` | `Release` | 200 `{"pocket": "queued" or "idle"}`; 409; 403 |
| `GET /consumers/{consumer_id}/lease` | none | the `Consuming` surface's `exposes` | 200 with the lease, or 204 when the consumer holds none; 404 `unknown_consumer` |
| `GET /healthz` | none | none | 200 |

Every error body is `{"error": "<snake_case variant>", "message": "<text>"}`, one variant
per `Error` variant plus `bad_request` for a malformed body; `expires_at` is the
`Timestamp`'s milliseconds. The mapping from `Error` to status lives in one function.

`crates/pachinko/tests/http.rs` starts `serve` in-process on port 0 with a short
`lease_ttl` and sweep, then with `ureq`: joins a publisher and a consumer; publishes three
records into two pockets, superseding one; pulls and checks the batch is oldest-first and
the pocket exclusive (a second consumer's pull gets the other pocket, a third gets
`no_pocket_available`); acks, nacks and releases, and checks the superseded version is
deferred and returns after release; sends an ack under the released lease and checks 409;
lets a lease lapse and polls `GET .../lease` until the sweep has freed it or a deadline
passes, failing on the deadline and never sleeping blind; closes the shutdown channel
and checks `serve` returns `Ok` within the grace period. `tests/binary.rs` gains one case:
`pachinko serve --listen 127.0.0.1:0` started as a child process exits 0 on SIGTERM.

#### Decision 0019 and the handbook

`docs/decisions/0019-tokio-and-axum-service.md`, slug `decision_service_runtime`, in the
shape decision 0014 has: Context (0014's trigger fired with T19's surfaces; the two
readings), the spike as a table (tokio with axum; plain threads with a hand-written HTTP/1.1
loop; `async-std` or `smol`), judged on the dependency tree `just deny` must accept, the
test story, maintenance, and fit with an effects boundary that keeps the runtime out of
the library; the Decision (tokio and axum; every crate with its 0007 reason; the
structured task scope as a rule; the lease id as a fencing token; the wire shape owned by
`docs/reference/http-api.md`); Consequences (the service's tree is now the audit surface,
no native TLS, the bare binary and the image unchanged, the library still synchronous);
What would reopen this; Related pages. Decision 0014 gains one line under its title,
"Superseded by decision 0019 on <date>", and nothing else changes in it;
`docs/decisions/README.md` gets 0019's row, marks 0014 superseded in the paragraph that
already marks 0004, and moves "The next decision is" to 0020. `docs/manifest.yml` registers
the record and the new page `docs/reference/http-api.md` (kind `reference`, slug
`http_api`).

The handbook pages that say the domain is undefined, the service exposes nothing or no
trait exists are brought up to date (`review-docs` skill). T19's Open points list the ten
stale places: `README.md`, `AGENTS.md` (the identity sentence and the invariant block),
`docs/README.md`, `docs/project/repository-map.md`, `docs/tutorials/first-change.md`,
`docs/reference/configuration.md`, `docs/reference/testing.md`,
`crates/libpachinko/src/lib.rs`, `crates/pachinko/README.md`; to those add
`docs/explanation/architecture.md` ("No trait exists yet", "The service today", "What is
deferred"), `docs/explanation/security-model.md` ("What is deferred": the listener is now
the surface), `docs/project/purpose-and-scope.md`, `docs/project/terminology.md` (the
Service row's "never server" clause and its citation of 0014; new rows for Clock and Lease
id if the page's domain table wants them), `docs/reference/testing.md` (the fakes exist;
proptest exists; the HTTP test), `docs/how-to/test-and-debug.md` (how to run `serve` and
drive it with `curl`), `docs/explanation/layering.md` (one module per concern inside the
library, the module order), `crates/libpachinko/README.md` and `CHANGELOG.md` (both
crates' Unreleased, Added). `AGENTS.md` gains invariant 10, "Errors are thiserror enums",
in the Invariants list, and its identity sentence says the domain is specified and
implemented; `just check-agents` must stay green, so B's six phrases are untouched.

## Goal

`libpachinko` implements every rule, transition, invariant and guarantee of
`docs/specs/pachinko.allium` behind a synchronous `Queue` API that takes its clock and its
lease ids through traits with fakes beside them; every obligation `just plan-spec pachinko`
lists maps to a test; `pachinko serve` runs a tokio HTTP server over an in-memory queue
whose routes mirror the two surfaces, with a lapse sweep and a clean shutdown; decision
0019 supersedes 0014; the handbook says the domain is implemented; `just check` is green.

## Non-goals

- No persistence, snapshot, replication, recovery or restart story. The queue lives in one
  process's memory and a restart empties it.
- No answer to any of the module's nine open questions. The equal-rank publish stays
  rejected, as the checker's placeholder has it; the response says it is a placeholder.
- No surface for reading the superseded and dead-letter queues (open question 2). Both are
  kept and decay; nothing exposes them.
- No capacity bound, shedding, disk spill or back-pressure (open question 3).
- No authentication, authorisation, TLS or rate limiting. `openssl-sys` stays denied.
- No client crate, no OpenAPI document, no second specification module, no change to
  `docs/specs/pachinko.allium`.
- No change to `Justfile`, `pyproject.toml`, the CI workflows or `Dockerfile`: the gate
  already covers everything this ticket adds, and the image's entrypoint keeps working
  because the bare binary keeps its behaviour (Steps, step 5).
- No publishing, tagging or release. Pushing and the pull request are separately
  authorised (`CONVENTIONS.md` §11).

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | shared, own lines | `[workspace.dependencies]` entries with decision-0007 notes |
| `Cargo.lock`, `fuzz/Cargo.lock` | shared, regenerated | `just lock` after the manifests |
| `crates/libpachinko/Cargo.toml` | shared, own lines | thiserror; proptest as a dev-dependency |
| `crates/libpachinko/src/lib.rs` | shared, own lines | Crate docs; the modules; re-exports |
| `crates/libpachinko/src/clock.rs` | new | `Clock`, `Timestamp`, `FakeClock` |
| `crates/libpachinko/src/lease_ids.rs` | new | `LeaseIds`, `LeaseId`, `CountingLeaseIds` |
| `crates/libpachinko/src/ids.rs` | new | The id, key, rank, payload and position newtypes |
| `crates/libpachinko/src/config.rs` | new | `Config`, defaults, the five load-time checks |
| `crates/libpachinko/src/error.rs` | new | The thiserror `Error` |
| `crates/libpachinko/src/queue.rs` | new | `Queue` and the rules; may split into `queue/` children by rule group, with `queue.rs` beside the directory, never `mod.rs` |
| `crates/libpachinko/tests/*.rs` | new | `publish.rs`, `pull.rs`, `settle.rs`, `lease.rs`, `expiry.rs`, `decay.rs`, `invariants.rs` |
| `crates/libpachinko/README.md` | shared, own lines | What the crate implements |
| `crates/pachinko/Cargo.toml` | shared, own lines | tokio, axum, serde, serde_json, base64, rand, tracing, tracing-subscriber; ureq as a dev-dependency |
| `crates/pachinko/src/lib.rs` | shared, own lines | `run(args, out)`: the version line and the `serve` dispatch |
| `crates/pachinko/src/main.rs` | shared, own lines | Still one line of behaviour; passes the arguments |
| `crates/pachinko/src/serve.rs` | new | The runtime, the task scope, the sweep, shutdown |
| `crates/pachinko/src/http.rs` | new | The router, handlers, bodies, the error mapping |
| `crates/pachinko/src/clock.rs` | new | `SystemClock` |
| `crates/pachinko/src/lease_ids.rs` | new | `RandomLeaseIds` |
| `crates/pachinko/src/config.rs` | new | The flags and their defaults |
| `crates/pachinko/tests/http.rs` | new | The end-to-end client test |
| `crates/pachinko/tests/binary.rs` | shared, own lines | The bare line unchanged; `serve` exits 0 on SIGTERM |
| `crates/pachinko/README.md` | shared, own lines | What the service does and how to run it |
| `AGENTS.md` | shared, own lines | Identity sentence; invariant 10 |
| `README.md`, `docs/README.md` | shared, own lines | The domain is implemented |
| `docs/manifest.yml` | shared, own lines | Decision 0019; `reference/http-api.md` |
| `docs/decisions/0019-tokio-and-axum-service.md` | new | The record |
| `docs/decisions/0014-service-runtime-deferred.md` | shared, own lines | The superseded line |
| `docs/decisions/README.md` | shared, own lines | The row; the superseded note; the next number |
| `docs/reference/http-api.md` | new | The route table and the error mapping |
| `docs/explanation/architecture.md`, `docs/explanation/security-model.md`, `docs/explanation/layering.md` | shared, own lines | The traits exist; the listener is the surface; the module order |
| `docs/project/purpose-and-scope.md`, `docs/project/repository-map.md`, `docs/project/terminology.md` | shared, own lines | Implemented, not only specified; the Service row |
| `docs/reference/testing.md`, `docs/reference/configuration.md`, `docs/tutorials/first-change.md`, `docs/how-to/test-and-debug.md` | shared, own lines | The fakes, proptest, the HTTP test, the config, running `serve` |
| `CHANGELOG.md` | shared, own lines | Both crates' Unreleased, Added |
| `tickets/README.md` | shared, own lines | The index row's status; the bootstrap paragraph |
| `tickets/T20-prototype-and-rest-api.md` | ticket | `status:` and hand-back notes |

Untouched on purpose: `Justfile`, `pyproject.toml`, `.github/**`, `Dockerfile`,
`docs/specs/pachinko.allium`, `tests/fixtures/effects-violation/**`,
`crates/libpachinko/clippy.toml`. A need to change any of them is handed back.

## Steps

1. Create the worktree from `main` after T19 has merged (`README.md`, "How to pick up a
   ticket") and run `just initialize` (network, authorised). Run `just check` on the
   untouched worktree and quote its last line as the baseline.
2. **The spike and decision 0019.** Use the `plan-change` skill. Read decision 0014, the
   two readings in Context, and the module's two surfaces. Re-read each crate's current
   version with `cargo search` (network, authorised). Write
   `docs/decisions/0019-tokio-and-axum-service.md` with its comparison table, mark 0014
   superseded, register the record and `docs/reference/http-api.md` in `docs/manifest.yml`
   and `docs/decisions/README.md`, and write the page from the route table in Design. Run
   `just check-docs`.
3. **The dependencies.** Add every crate through `[workspace.dependencies]` with its
   decision-0007 note, then `just lock` (network, authorised). Run `just lock-check`,
   `just deny`, `just deps-unused` and `just effects`; quote the `cargo tree` line
   `just effects` reads for libpachinko, and `cargo tree -p pachinko -e normal --depth 1
   --locked --offline` for the service, so the hand-back shows tokio on the right side of
   the boundary. `deps-unused` will be red until step 4 uses each crate; it is quoted
   green at the end of step 5.
4. **The library.** Use the `rust-change` skill and, for the tests, the `propagate` skill
   over `just plan-spec pachinko`. Write the traits and fakes, the ids, the config and the
   error type; derive the integration tests and the proptest walk from the obligations and
   show them failing; then implement `Queue` rule group by rule group in the module's
   order: identity, publish and revive, pull and hold, ack and nack, renew and release,
   lapse, decay, the `exposes` read. Run `just fmt-check`, `just clippy`, `just test`,
   `just effects`, `just coverage` and `just doc`; quote the reconciliation line.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint 1, over the library diff. Ask:
   whether every rule of the module is implemented by exactly the operation that names it
   and no operation does what no rule says; whether the trace in T19's pull request
   description runs through the code with the same states at every step; whether any
   sequence of operations can violate a module invariant (the proptest walk is the
   evidence, and the reviewer looks for a generator that cannot reach a state); whether a
   temporal test depends on wall-clock time; whether a direct `std` effect slipped past
   clippy behind a `cfg`; whether coverage is met by tests that assert something.
5. **The service.** Use the `rust-change` skill. `run(args, out)`, `serve`, `SystemClock`,
   `RandomLeaseIds`, the router and the error mapping, the sweep, the shutdown, the task
   scope, `tests/http.rs` and the `binary.rs` case. Run `just fmt-check`, `just clippy`,
   `just test`, `just coverage`, `just doc`, `just deps-unused` and `just effects`. Run the
   binary by hand once, `cargo run -p pachinko --locked -- serve`, and the `curl` lines in
   Verification against it; quote them.
6. **The handbook ripple.** Use the `review-docs` skill over every page Design names and
   `CHANGELOG.md`. Run `just check-docs` and `just check-agents`.
7. **Alignment.** Use the `weed` skill over `docs/specs/pachinko.allium` against both
   crates. A divergence is fixed in code; one the code cannot fix without a product
   decision is an Open point naming the rule, never a code judgement and never a spec
   edit.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint 2, over the whole diff before
   the final commit. Ask: whether every `provides` of the two surfaces has exactly one
   route and every `exposes` item is in the `GET` body; whether a pull can return an HTTP
   error for anything but an unknown consumer or a malformed body; whether any task is
   spawned outside the one scope, or outlives `serve`; whether a mutex is held across an
   `.await`; whether a late renew, ack or release and the sweep can interleave so that a
   rule's guard is read in one state and its effects applied in another; whether the
   stale-lease rejection holds when the same consumer holds the same pocket again; whether
   the bare binary and the image's entrypoint behave exactly as before; whether a page
   outside Files touched now says something false.
8. `just check`, then commit on the ticket branch, with the hand-back notes quoting each
   output headed by the commit it ran at. Before asking to push, `git fetch origin`
   (network, authorised), merge or rebase onto `origin/main`, rerun and requote. Pushing
   and the pull request are separately authorised: stop and ask.

## Acceptance criteria

- `just plan-spec pachinko` exits 0; its obligation count and the `propagate`
  reconciliation line are quoted, with zero uncovered obligations or each uncovered one
  classified with its reason.
- `just effects` is green; the quoted trees show `tokio` in pachinko's direct dependencies
  and absent from libpachinko's.
- `just test` is green, and the hand-back quotes the failing run of the derived tests from
  before the implementation.
- `crates/pachinko/tests/http.rs` proves, against a server started in-process: joins,
  the four publish outcomes, an oldest-first batch, pocket exclusivity, ack, nack, renew,
  release, a deferred version returning after release, a 409 for an action naming a
  released lease, a lapse freed by the sweep, and a clean shutdown.
- `crates/pachinko/tests/binary.rs` is green with its first case unchanged; the bare
  binary still prints `pachinko <version>` and nothing else, so the image's
  `docker run --rm pachinko:dev` is unchanged.
- No `tokio::spawn` or `std::thread::spawn` outside `crates/pachinko/src/serve.rs`, and
  every handle it creates is joined before `serve` returns (quote `grep -rn
  'spawn' crates/`).
- Decision 0019 exists and is registered; decision 0014 carries the superseded line;
  `docs/reference/http-api.md` exists and holds the route table; `just check-docs` and
  `just check-agents` are green.
- `AGENTS.md` carries invariant 10 and `just check-agents` prints
  `Validated AGENTS.md, 2 adapters, and 14 skills.`
- No open question is answered: the module is byte-identical to `main`'s, an equal-rank
  publish returns 409 with a message naming open question 1, and the hand-back's Open
  points record every reading the prototype took.
- Both adversarial reviews ran (`CONVENTIONS.md` §14), and every valid finding is fixed or
  handed back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing after the commit.

## Verification

```sh
just plan-spec pachinko
just effects
cargo tree -p pachinko -e normal --depth 1 --locked --offline
just test
just coverage
just check-docs
just check-agents
just check
git status --short
```

The smoke test by hand, against `cargo run -p pachinko --locked -- serve` in another
terminal, each reply quoted:

```sh
curl -s -X POST localhost:8080/publishers/p1
curl -s -X POST localhost:8080/consumers/c1
curl -s -X PUT localhost:8080/pockets/tenant-a/records/k1 \
  -H 'content-type: application/json' \
  -d '{"publisher_id":"p1","rank":1,"payload":"aGVsbG8="}'
curl -s -X PUT localhost:8080/pockets/tenant-a/records/k1 \
  -H 'content-type: application/json' \
  -d '{"publisher_id":"p1","rank":2,"payload":"d29ybGQ="}'
curl -s -X POST localhost:8080/consumers/c1/pull \
  -H 'content-type: application/json' -d '{"limit":10}'
curl -s -X POST localhost:8080/leases/<lease_id>/ack \
  -H 'content-type: application/json' -d '{"consumer_id":"c1","record_key":"k1"}'
curl -s -X POST localhost:8080/leases/<lease_id>/release \
  -H 'content-type: application/json' -d '{"consumer_id":"c1"}'
curl -s -X POST localhost:8080/leases/<lease_id>/ack \
  -H 'content-type: application/json' -d '{"consumer_id":"c1","record_key":"k1"}'
```

The last line must answer 409 `lease_not_active`.

## Hand-back notes

Written by the executing agent, each quoted output headed by the commit it ran at
(`CONVENTIONS.md` §11).

## Open points

Prewritten for the maintainer; the executing agent adds to them and strikes none.

- **One ticket or two.** The library with its tests and the service with its REST API and
  decision 0019 could be two tickets with the first merged first. The maintainer asked for
  one; the two review checkpoints mark where it would split.
- **The lease-id source.** A trait with a random 128-bit token in the service is the
  design, so that a lease id is unguessable and randomness stays out of the library. A
  counter in the library would be simpler and would serve the fencing argument, which
  needs only that an ended lease's id is never reissued; it is not unguessable. The
  maintainer confirms the trait.
- **Open question 8 in the prototype.** A late renew, ack, nack, pull or release that
  reaches the mutex before the sweep does extends the lease, because the rules guard on
  status alone; one that reaches it after is rejected. The prototype records this as the
  processing order, as the module says, and decides nothing.
- **The wire shape is a prototype's.** JSON with base64 payloads, keys in the path, the
  lease id in the path, the consumer id in the body. A production API might carry the
  payload as the body with the keys in headers, or take a different stance on
  idempotency; the maintainer replaces the page and the handlers, not the library.
- **Flags, not a config file.** The service reads its listen address and the module's
  five config values from flags with the module's defaults; no file, no environment
  variable. A deployment story is a later ticket.
- **`tracing` is optional for a proof of concept.** It is in the table because the lint
  table forbids `eprintln!`; the maintainer may prefer silence and two fewer crates.
- **The handbook's "consumer" in the generic sense** (T19's Open points) is still open and
  not this ticket's: it rewords only the pages Design names.
- **The six checker limits** T19 handed back for `docs/how-to/work-with-the-specs.md` are
  still handed back; this ticket does not edit that page.
