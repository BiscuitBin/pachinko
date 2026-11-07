---
id: T24
title: "Service tracer: the owner task and HTTP join, publish one record, pull"
status: todo
depends_on: [T23]
parallel_with: []
branch: ticket/t24-service-http-tracer
estimated_size: L
---

# T24: Service tracer: the owner task and HTTP join, publish one record, pull

## Context

T23 landed the library tracer: the `Clock` and `LeaseIds` traits with their fakes, the
`PocketRecords` seam with `MemoryRecords`, and a `Queue` that joins, publishes and pulls.
`pachinko` is still the bootstrap's one-line binary. This ticket makes it a service: a
tokio runtime, one owner task that holds the `Queue` and applies every command in
arrival order, the real clock and lease-id source, and an axum HTTP/1.1 JSON API over the
join, single publish, pull and lease-view actions. Decision 0019 (T22) chose the shape;
the decision table in T22's Context is the authority and this ticket restates only what
it builds: the wire is PR 39's routes and error map; the executor is one owner task over
a bounded `mpsc` channel with a `oneshot` reply per command and no `Mutex`, so that a
lapse (T26) and a consumer's action are totally ordered; lease ids are an epoch drawn once
from the operating system's random source and a serial; the clock is `SystemTime` behind
the queue's monotonic clamp. The lease actions (T25), the tick lane (T26), the batch route
and the request-shape limits (T27) and readiness, logging, timeouts and shutdown grace
(T29) each build on the shapes this ticket fixes, which is why it carries a design review
mid-way.

The prototypes this ticket reads are heads of unmerged branches; a fresh worktree needs
`git fetch origin` (network, authorised by step 2) before `git show` can read them. Never
check a prototype branch out into the ticket's worktree; read with `git show <head>:<path>`.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 38, the Flight service prototype | `tickets/t20-pachinko-prototype-complete` | `cb77f07` | `git show cb77f07:<path>` |
| PR 39, the HTTP service prototype | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` | `git show cf1a6b2:<path>` |

The files to read: PR 38 `crates/pachinko/src/actor.rs` (`COMMAND_CAPACITY` line 40,
`Command` line 50, `QueueHandle` line 120, `start` line 281, `apply` line 329); PR 39
`crates/pachinko/src/http.rs` (the routes at lines 43 to 52, `status` line 89, `code`
line 104, `log_request` line 413), `crates/pachinko/src/config.rs` (the flags at lines
125 to 131), `crates/pachinko/src/serve.rs` (`scope` line 102, `drain` line 146,
`Connections` line 200, `install_signals` line 346), `crates/pachinko/src/lib.rs`,
`crates/pachinko/tests/http.rs` (`Client` line 26), `crates/pachinko/tests/binary.rs`
(`serve_exits_zero_on_sigterm` line 36), root `Cargo.toml` (the 0007 notes),
`docs/reference/http-api.md` and `docs/how-to/test-and-debug.md` (the curl walkthrough).
Where this ticket departs from PR 39 it says so, because the executing agent will
otherwise copy it: no `Mutex` and no `with_queue`; no sweep and no `--sweep-ms` (T26); a
no-pocket pull answers `{"outcome": "no_pocket"}`, not `{"no_pocket_available": true}`;
the lease-id source refuses to start when the random source fails instead of falling
back to a counter; the request log line is `debug`, not `info`; the shutdown grace is a
constant until T29 makes it a flag and bounds the join.

## Goal

`pachinko serve` listens on `--listen`, answers the six routes below over one owner
task, and stops cleanly on SIGINT or SIGTERM; `just check` is green. This ticket owns:

| Owned | Items |
| --- | --- |
| Routes (6) | `POST /publishers/{publisher_id}`, `POST /consumers/{consumer_id}`, `PUT /pockets/{pocket_key}/records/{record_key}`, `POST /consumers/{consumer_id}/pull`, `GET /consumers/{consumer_id}/lease`, `GET /healthz` |
| Flags (6) | `--listen` (default `127.0.0.1:8080`), `--lease-ttl-ms` (300000), `--max-consecutive-expiries` (3), `--max-pull-batch` (100), `--superseded-ttl-ms` (43200000), `--dead-letter-ttl-ms` (43200000) |
| Error codes (6) | `bad_request` 400, `publisher_exists` 409, `consumer_exists` 409, `unknown_publisher` 404, `unknown_consumer` 404, `equal_rank` 409 |

It exercises over HTTP, without owning them (T23 owns the library tests):

| Id | Description |
| --- | --- |
| S019 | Large ranks keep full precision across supersede and stale |
| S020 | Payload bytes and keys round-trip unchanged |
| S028 | A positive limit returns at most that many, oldest first, and later pulls continue |
| S031 | A zero-limit pull claims a pocket and returns an empty batch |
| S033 | A pull with no queued pocket returns `NoPocketAvailable`, not an error |

It pre-lands one non-domain code it does not own: 503 `not_ready`, the answer of a
handler whose owner has stopped; T29 owns `GET /readyz` and the code.

## Non-goals

- No ack, nack, renew or release route, and no `lease_not_active`, `lease_not_yours` or
  `record_not_held` code: T25.
- No sweep, no `Tick` lane and no `--sweep-ms`: T26. A lease granted here never lapses.
- No batch route and no request-shape limits: T27. The single `PUT` ships with axum's
  default body limit of 2 MiB, answered 400 `bad_request`, and `http-api.md` says so.
- No `GET /readyz`, `--log-level`, `--request-timeout-ms` or `--shutdown-grace-ms`, and
  no bounded join on shutdown: T29. The container entrypoint still prints the version;
  T29 documents running `serve` in the container as run arguments.
- No change to any library rule, and no library change at all unless step 4 finds a
  missing accessor, which is then one `pub fn` under `rust-change`, listed in the
  hand-back.
- No authentication, TLS or rate limiting; the surface is loopback by default.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | shared, own lines | The decision-0007 note and full `x.y.z` caret version for each crate in the dependency step, under `[workspace.dependencies]` |
| `Cargo.lock` | shared, own lines | Regenerated by `just lock` |
| `crates/pachinko/Cargo.toml` | shared, own lines | The `{ workspace = true }` lines under `[dependencies]` (`thiserror` among them, first used here) and `[dev-dependencies]` |
| `crates/pachinko/src/lib.rs` | replaced | Crate docs; `run(args, out)`, `exit_code`; the module list and re-exports |
| `crates/pachinko/src/main.rs` | shared, own lines | Still one statement: `run` over the process arguments and `exit_code` over its result |
| `crates/pachinko/src/error.rs` | new | `ServeError` |
| `crates/pachinko/src/clock.rs` | new | `SystemClock` |
| `crates/pachinko/src/lease_ids.rs` | new | `EpochSerialLeaseIds` |
| `crates/pachinko/src/config.rs` | new | `ServeConfig` and the six flags |
| `crates/pachinko/src/actor.rs` | new | `Command`, `QueueHandle`, the owner loop |
| `crates/pachinko/src/http.rs` | new | The router, the six handlers, `Refusal` and the exhaustive map |
| `crates/pachinko/src/serve.rs` | new | The runtime, the task scope, signals, graceful shutdown |
| `crates/pachinko/tests/http.rs` | new | The wire tests against an in-process server |
| `crates/pachinko/tests/binary.rs` | replaced | The version line, a usage error, `serve` on port 0 |
| `docs/reference/http-api.md` | new | The route table, the body shapes, the error table |
| `docs/manifest.yml` | shared, own lines | The row for `reference/http-api.md`, `canonical_for: ["http_api"]` |
| `docs/README.md` | shared, own lines | The "Look up" entry for the HTTP API |
| `docs/how-to/test-and-debug.md` | shared, own lines | A "Run the service" section with the curl walkthrough |
| `docs/reference/configuration.md` | shared, own lines | The intro sentence T22 left pointing here; a "Service flags" section |
| `docs/explanation/architecture.md` | shared, own lines | "The service today" becomes "The service" |
| `docs/explanation/security-model.md` | shared, own lines | "What is deferred" becomes the network surface as built |
| `CHANGELOG.md` | shared, own lines | pachinko Unreleased, Added |
| `crates/pachinko/README.md` | replaced | What the service is, how to run it |
| `tickets/README.md` | shared, own lines | The index row's status |
| `tickets/T24-service-http-tracer.md` | ticket | `status:` and hand-back notes |

`docs/how-to/build-the-container-image.md` and `Dockerfile` are read and left alone:
the entrypoint is `/pachinko` with no arguments, which prints the version as before, and
T29 documents `serve` as the container's run arguments without changing it.
`docs/reference/testing.md` is T28's to extend with the catalog; this ticket's test
layout is described in `architecture.md`.

## Steps

1. Create the worktree from `main` after T23 has merged (`README.md`, "How to pick up a
   ticket"), or, in a clean worktree already initialized on `main` whose branch carries
   nothing, create the branch there and skip `just initialize`. Run `just check` on the
   untouched worktree and quote its last line as the baseline.
2. **The plan.** Use the `plan-change` skill. Run `git fetch origin` (network, authorised
   by this step) so the prototype heads in Context resolve, and read the files listed
   there with `git show`. Read T23's public API as merged (`crates/libpachinko/src/`:
   `clock.rs`, `lease_ids.rs`, `ids.rs`, `config.rs`, `error.rs`, `queue.rs`) and write
   the plan against the names it shipped, not the names this ticket uses where they
   differ; the hand-back names each difference.
3. **The dependencies.** Each crate below takes a decision-0007 note in
   `[workspace.dependencies]` of the root `Cargo.toml`, in the comment form the file
   already uses for divan, a full `x.y.z` caret version, and a `{ workspace = true }` line
   in `crates/pachinko/Cargo.toml`. The versions are the ones PR 39 pinned; re-pin each
   to the current release and write the full `x.y.z`. `thiserror` is already in the
   workspace table from T23, which added it to `libpachinko` only; this ticket adds the
   `{ workspace = true }` line to `crates/pachinko/Cargo.toml` when `ServeError` first
   derives it, and `just deps-unused` is the check that it is used.

   | Crate | Kind | Features | 0007 note, gist |
   | --- | --- | --- | --- |
   | `thiserror` 2.0.21 | normal, manifest line only | defaults | T23's workspace note stands: derives `ServeError` (invariant 10) |
   | `tokio` 1.54.1 | normal | `rt-multi-thread`, `macros`, `net`, `sync`, `time`, `signal` | the runtime, the listener, timers, signals and the command channels; pachinko only, `just effects` keeps it out of the library |
   | `axum` 0.8.9 | normal | `http1`, `json`, `tokio`, no defaults | routes HTTP to the owner; without it the service hand-writes hyper glue |
   | `serde` 1.0.229 | normal | `derive` | the request and reply bodies; the library stays serde-free |
   | `base64` 0.23.1 | normal | defaults | payload bytes on a JSON wire |
   | `getrandom` 0.4.3 | normal | defaults | the lease-id epoch, one call at start; a smaller tree than rand |
   | `tracing` 0.1.44 | normal | `std`, no defaults | log lines, which the lint table forbids `eprintln!` from writing |
   | `tracing-subscriber` 0.3.23 | normal | `fmt`, `std`, no defaults | the subscriber on standard error |
   | `serde_json` 1.0.154 | dev | defaults | reads replies in the wire tests |
   | `ureq` 3.5.0 | dev | no defaults, no TLS | the blocking client the wire tests use |

   Run `just lock` (network, authorised by this step), then `just lock-check`,
   `just deny` and `just deps-unused`. A licence `just deny` names that is absent from
   `deny.toml`'s allow list is a finding to record in the hand-back with the crate that
   carries it, not an edit to `deny.toml`; every crate above is MIT or Apache-2.0 at the
   pinned versions, so the expected outcome is green.
4. **The effects and the flags.** Use the `rust-change` skill (decisions 0003 and 0008:
   the trait is the library's, the real implementation is here). Unit tests stay in the
   module they test (decision 0009); every public item takes a doctest; public types are
   `Send + Sync + 'static`, `Clone`, `Debug` and `#[non_exhaustive]` (invariant 3).
   - `error.rs`: `ServeError`, a thiserror enum: `Usage(String)` (a command line the
     binary does not read), `Config(libpachinko::ConfigError)` (a flag set the library
     refuses), `Io { what: &'static str, message: String }` (what the service was doing
     and the I/O error's text: `std::io::Error` is not `Clone`, so the error travels as
     text to keep invariant 3, as PR 39 `crates/pachinko/src/error.rs` did, and a
     `pub(crate) fn io(what)` builds it for `map_err`), `Task(String)` (a task panicked
     or was cancelled), `ClockBeforeEpoch`, `RandomSource(String)` (the operating
     system's random source failed) and `LeaseIdsExhausted`. `Display` of `Usage` is
     `usage: <text>`, so the binary's standard error reads
     `pachinko: usage: unknown flag --bogus`, and of `Io` is `<what>: <message>`.
   - `clock.rs`: `SystemClock`, `Clock` over `SystemTime::now()` to `Timestamp` in
     milliseconds since the Unix epoch. `SystemClock::new()` reads the clock once and
     returns `ClockBeforeEpoch` when it is before the epoch; `now()` afterwards
     saturates to zero, since the trait is infallible and the queue's clamp never moves
     backward anyway.
   - `lease_ids.rs`: `EpochSerialLeaseIds`, `LeaseIds`. `new()` draws 8 bytes with
     `getrandom::fill` once, as the epoch, and returns `RandomSource` when that fails;
     `serve` refuses to start on that error. The serial is an `AtomicU64` starting at 1,
     advanced with `checked_add`; `next()` returns `LeaseId::from_u128((u128::from(epoch)
     << 64) | u128::from(serial))`, and the wire shows it as the 32-hex-digit `Display`
     T23 defined. A serial that cannot advance leaves the source exhausted: `exhausted()`
     reports it, the owner (step 5) checks it before applying any `Pull` and stops with
     `LeaseIdsExhausted` rather than reissue an id. A `#[cfg(test)]` constructor starts
     the serial where the test says, so the exhaustion test needs two pulls, not 2^64.
   - `config.rs`: `ServeConfig::parse(args)` over `--name value` pairs, as PR 39
     `crates/pachinko/src/config.rs` lines 125 to 131 reads them, with the six flags
     in Goal and their defaults; `listen()` and `queue()` accessors. An unknown flag, a
     flag without a value, a value that is not an unsigned integer (so `-1` and `x` both
     fail) are `Usage` errors with the message naming the flag; `--max-consecutive-expiries 0`
     and `--max-pull-batch 0` reach the library's config check and come back as
     `Config` (`ExpiryLimitPositive`, `PullBatchPositive`). T26, T27 and T29 add their
     flags to this parser.
5. **The owner task, the handle, the route and error-map shape.** Use the `rust-change`
   skill.
   - `actor.rs`, after PR 38 `crates/pachinko/src/actor.rs` line 50: `Command` is an
     enum with one variant per action and a `oneshot::Sender` reply in each:
     `PublisherJoins { publisher_id, reply: Reply<Result<(), JoinError>> }`,
     `ConsumerJoins`, `Publish { publisher_id, publication, reply:
     Reply<Result<PublishOutcome, PublishError>> }`, `Pull { consumer_id, limit: i64,
     reply: Reply<Result<PullOutcome, PullError>> }`, `LeaseView { consumer_id, reply:
     Reply<Result<Option<LeaseView>, PullError>> }`, the shape T23's `lease_view`
     returns, so the lease read answers `unknown_consumer` for a consumer that never
     joined and `None` for one holding no lease. T25 adds the four actions, T26 the
     `Tick` lane, T27 `PublishBatch`. The channel is `mpsc::channel(1024)`
     (`COMMAND_CAPACITY`). `QueueHandle` is `Clone` over the sender; each method builds
     a command around a fresh reply channel, `send(...).await`s (a full channel makes
     the sender wait: back-pressure, never a refusal) and awaits the answer; a closed
     inbox or a dropped reply is `Stopped`, one variant, which the handler turns into
     503 `not_ready`. The owner loop is `while let Some(command) = inbox.recv().await`,
     applying each command synchronously against `Queue<MemoryRecords, SystemClock,
     EpochSerialLeaseIds>`: no owner borrow crosses an `.await`. Before applying a `Pull`
     the loop checks `reply.is_closed()` and skips the command with a `debug` line when
     the requester has gone: the lease must not be granted to nobody. A requester that
     goes between that check and the grant keeps its lease until it lapses (T26); the
     docs say so. The owner runs in the `JoinSet` that `serve` owns and ends when every
     sender is dropped; it returns `Err(LeaseIdsExhausted)` as above. `answer` drops a
     failed reply send, as PR 38 line 379 does.
   - `http.rs`, after PR 39 `crates/pachinko/src/http.rs` lines 43 to 52: the router
     over an `AppState` holding the `QueueHandle`, with the join handlers only at this
     step, and the error map in full. `Refusal` is the service's own enum, one variant
     per code on the wire: `PublisherExists`, `ConsumerExists`, `UnknownPublisher`,
     `UnknownConsumer`, `EqualRank { pocket_key, record_key, rank }`, `BadRequest(String)`,
     `NotReady`; `status()` and `code()` match it exhaustively with no wildcard arm, so
     a variant T25 or T27 adds without a mapping does not build. The library's enums are
     `#[non_exhaustive]` (invariant 3), so the `From<JoinError>`, `From<PublishError>`
     and `From<PullError>` conversions each carry the one arm the attribute forces; that
     arm maps to `NotReady` with a message naming the variant and an `error` log line,
     and the per-variant unit test below proves it unreached for every variant the
     library has today. `IntoResponse` for `Refusal` writes `{"error": <code>,
     "message": <text>}` with the status. A JSON body that cannot be read, a lease id
     that is not 32 hex digits and a payload that is not base64 are `BadRequest`, in
     place of axum's plain-text rejection. The request log line (PR 39 line 413) is
     emitted at `debug`, so the default subscriber at `info` prints none: the per-request
     `info` line cost PR 39 a third of its publish rate, which the evidence table in
     T22's Context records.

   **Adversarial review** (`CONVENTIONS.md` §14): the uncommitted diff of steps 3 to 5
   against this ticket's Goal, Files touched and the decision table in T22's Context,
   PR 38 `crates/pachinko/src/actor.rs` and PR 39 `crates/pachinko/src/http.rs`. The
   `Command` enum and its reply types, the `Refusal` enum and the exhaustiveness of
   `status()` and `code()`, and the handle's back-pressure are what T25, T27 and T29
   build on. Ask: whether a reply can be dropped so that a lease is granted to nobody,
   and whether the `is_closed` check leaves a window the docs do not state; whether any
   handler holds anything across an `.await` that the owner also needs; whether the map
   can compile with a variant unmapped, counting the arms `#[non_exhaustive]` forces;
   whether a full channel can refuse rather than wait; whether `EpochSerialLeaseIds` can
   reissue an id or the owner can grant one after exhaustion; whether any public type
   misses invariant 3; and whether a dependency, feature or version departs from the
   table in step 3. Codex `gpt-6-astra` reviewers run read-only with fresh contexts, one
   lens each; separate triage subagents write verdicts; valid fixes are applied one
   writer at a time; two rounds at most.
6. **The remaining handlers, the server and the binary.** Use the `rust-change` skill.
   - The handlers, in `http.rs`. Keys travel in the path and are percent-decoded by
     axum's `Path`; bodies are JSON; payloads are base64 (the standard alphabet with
     padding); `expires_at` is milliseconds since the Unix epoch; `rank` is a JSON
     integer read as `i64`, so `9223372036854775807` round-trips (S019). The shapes:

     | Route | Request | Replies |
     | --- | --- | --- |
     | `POST /publishers/{publisher_id}` | no body | 201 no body; 409 `publisher_exists` |
     | `POST /consumers/{consumer_id}` | no body | 201 no body; 409 `consumer_exists` |
     | `PUT /pockets/{pocket_key}/records/{record_key}` | `{"publisher_id": "p1", "rank": 1, "payload": "aGVsbG8="}` | 200 `{"outcome": "created"}`, or `"new_key"`, `"superseded"`, `"stale"`; 404 `unknown_publisher`; 409 `equal_rank` (the OQ1 placeholder; the message says so); 400 `bad_request` |
     | `POST /consumers/{consumer_id}/pull` | `{"limit": 10}` | 200 `{"outcome": "batch", "lease_id": "<32 hex digits>", "pocket_key": "tenant-a", "expires_at": 1762500000000, "records": [{"record_key": "k1", "rank": 1, "payload": "aGVsbG8="}]}`, oldest first, or 200 `{"outcome": "no_pocket"}`; 404 `unknown_consumer`; 400 `bad_request` |
     | `GET /consumers/{consumer_id}/lease` | no body | 200 `{"pocket_key": "tenant-a", "expires_at": 1762500000000, "records": [{"record_key": "k1", "rank": 1, "payload": "aGVsbG8="}]}`, the held records at the rank and payload they were handed out with; 204 no body when the consumer holds no lease; 404 `unknown_consumer` |
     | `GET /healthz` | no body | 200 `ok` as text |

     A no-pocket pull is data, not an error. A `limit` of zero or less claims the
     pocket and returns `"records": []` (S031). The pull reply, `expires_at` included,
     is built from the one `PullOutcome` the owner returns (T23's `Batch` carries it);
     a handler never sends a second `LeaseView` command to complete a reply, because a
     tick (T26) or another request (T25) could run between the two. `unknown_consumer`
     is answered before any lease guard, which matters once T25 adds the actions. A
     handler whose owner has stopped answers 503 `not_ready`.
   - `serve.rs`, after PR 39 `crates/pachinko/src/serve.rs`: `run(config, out)` installs
     the `tracing-subscriber` `fmt` writer on standard error at `info`, builds the
     multi-thread runtime, constructs `SystemClock` and `EpochSerialLeaseIds` (either
     error ends `run` before anything listens), binds `--listen`, installs the SIGTERM
     and SIGINT handlers (`install_signals` line 346) before it writes
     `listening on <address>` to `out`, so a signal sent the moment the line appears is
     not lost, then runs the scope. The public `serve(listener, &config, stop)` is what
     the wire tests call, on a listener they bound to `127.0.0.1:0`. The scope (line
     102) is one `JoinSet` holding the owner, the listener (`axum::serve` with graceful
     shutdown over PR 39's `Connections` and `Connection`, line 200, so that a
     connection open past the grace period is cut) and the signal task; the first to
     end starts the shutdown; the grace is a constant of 5 seconds (T29 makes it
     `--shutdown-grace-ms` and bounds the final join); the owner ends when the listener
     has dropped the last handle. `serve` returns the first error, and a panicked owner
     comes back from the `JoinSet` as `Task`, so `run` fails and `exit_code` is failure.
   - `lib.rs` and `main.rs`: `run(args, out)` writes `pachinko <version>` for no
     arguments, runs `serve` for `serve <flags>`, and is `Usage` for anything else;
     `exit_code(result, err)` writes `pachinko: <error>` to `err` and maps to the
     process's exit code. `main.rs` stays one statement over the process arguments,
     standard output and standard error. The crate docs describe the service.
7. **The tests.** Decision 0009 for placement; `just test` and `just coverage` run with
   `--all-features`.
   - Unit, `http.rs`: one test per code in Goal, plus `not_ready`, each asserting the
     status and the `{"error", "message"}` body of the `Refusal` it maps; a test that
     every variant of each library enum converts to its own `Refusal`, never to the
     forced arm.
   - Unit, `actor.rs`: a `Pull` whose reply receiver was dropped before the owner ran
     is skipped, shown by a second consumer then claiming the pocket (the owner is
     started after both commands are queued, so the order is fixed); a command to a
     stopped owner is `Stopped`; a source constructed one id from exhaustion grants one
     pull and the owner then ends with `LeaseIdsExhausted`; the owner applies commands
     in arrival order.
   - Unit, `config.rs`, `clock.rs`, `lease_ids.rs`, `serve.rs`: the defaults, every
     flag, each usage error's message, the two config refusals; `now()` is at or after
     the epoch; two ids differ and share the epoch; `serve` returns `Task` when the
     owner panics (the `pub(crate)` scope takes the owner's task, and the test passes
     one that panics) and `exit_code` of that is failure, which together are the
     process-level claim.
   - Wire, `crates/pachinko/tests/http.rs`, after PR 39 `crates/pachinko/tests/http.rs`
     line 26: a blocking `ureq` client against `serve` on `127.0.0.1:0` inside the
     test's own runtime, stopped through the `stop` channel. Each scenario names its id
     in a `// scenario: S0NN` comment and the obligations it exercises in an
     `// obligation:` comment, by the ids `just plan-spec pachinko` prints: S019 (ranks
     `9223372036854775807` and `-9223372036854775808` supersede and go stale exactly,
     read back from the batch); S020 (all 256 byte values in one payload, and keys
     carrying spaces, slashes and non-ASCII letters percent-encoded in the path, come
     back equal); S028 (three records, `limit` 2 returns the two oldest, the next pull
     under the lease returns the third); S031 (`limit` 0 returns `"records": []` and
     `GET .../lease` then shows the pocket held); S033 (a fresh consumer's pull is 200
     `{"outcome": "no_pocket"}`). Also: a second join of each id is 409; a publish by an
     unjoined publisher is 404; a pull and a lease read by an unjoined consumer are 404;
     an equal rank is 409 `equal_rank`; malformed JSON, a non-base64 payload and a
     missing field are 400 `bad_request`; a consumer with no lease reads 204;
     `GET /healthz` is 200. Every wait is bounded and fails on timeout; no assertion is
     satisfied by a timeout.
   - Binary, `crates/pachinko/tests/binary.rs`: the version line and empty standard
     error for no arguments; `serve --bogus 1` exits 1 with
     `pachinko: usage: unknown flag --bogus` on standard error and nothing on standard
     output; `serve --listen 127.0.0.1:0` writes `listening on 127.0.0.1:<port>`, the
     test then gets `/healthz` from that port and reads `ok` (so a server that never
     listened fails the test), sends SIGTERM and sees exit status 0 within a bounded
     wait that fails on timeout.
   - Run `just effects` (tokio and axum are in `pachinko` only), `just test`,
     `just coverage` (the floor is 90% of lines over `crates/*/src/**`; the service's
     signal and cut paths are the lines most likely to fall short, and the binary and
     wire tests above are where they are covered) and `just doc`.
8. **The handbook.** Use the `review-docs` skill. `docs/reference/http-api.md` is new,
   adapted from PR 39's page: frontmatter `kind: "reference"`, audience user,
   contributor, maintainer, operator and agent, `canonical_for: [http_api]`; the route
   table from step 6; the batch and lease shapes; `lease_id` as 32 hex digits, named by
   the actions T25 adds; the error table from Goal plus `not_ready`; the body limit
   sentence (2 MiB, axum's default, 400 `bad_request`, replaced by T27); "Not exposed"
   (the superseded and dead-letter queues, open question 2; a publish never refused for
   room); related pages. Register it in `docs/manifest.yml` and in `docs/README.md`
   "Look up". `docs/how-to/test-and-debug.md`: a "Run the service" section with the
   curl walkthrough for the six routes (join, publish `aGVsbG8=`, pull, read the lease,
   `healthz`; the ack and release lines arrive with T25). `docs/reference/configuration.md`:
   the intro sentence T22 left pointing at this ticket now says the flags are here, and
   a "Service flags" section lists the six with their defaults and says T26, T27 and T29
   add theirs. `docs/explanation/architecture.md`: "The service today" becomes
   "The service": the runtime, the owner task and the handle, the real `SystemClock` and
   `EpochSerialLeaseIds` behind the library's traits, the test layout (unit tests in the
   modules, `tests/http.rs`, `tests/binary.rs`). `docs/explanation/security-model.md`:
   "What is deferred" becomes the network surface as built: loopback by default,
   `--listen` opens it, no authentication, no TLS, the only body bound is axum's
   default until T27's limits. `CHANGELOG.md`: pachinko Unreleased, Added.
   `crates/pachinko/README.md`: what the service is and how to run it. Run
   `just check-docs`. Then the manual smoke run, outside the Verification block because
   `serve` does not return: `cargo run -p pachinko --locked -- serve --listen 127.0.0.1:0`
   in one terminal, the walkthrough from `test-and-debug.md` against the announced
   address in another, then ctrl-c, which exits 0; quote the output in the hand-back.
9. **The review.** Use the `review-change` skill over the whole diff against `main`.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, uncommitted, against
   this ticket's Goal, Files touched and Acceptance criteria, the decision table in
   T22's Context and `docs/specs/pachinko.allium`. Ask: whether a wire test passes
   against the library alone, that is, without the server having answered (a scenario
   whose assertions never read a reply); whether a binary test passes when the server
   never listened; whether a signal test passes on timeout; whether a test tagged with a
   scenario id exercises that scenario's description; whether a handler answers a
   refusal under a code the error table does not carry; whether a page outside Files
   touched now says the service listens on nothing, has no flags or has no HTTP stack;
   whether coverage is counted on test code; and whether the lockfile, the manifests
   and the 0007 notes agree. Codex `gpt-6-astra` reviewers run read-only with fresh
   contexts, one lens each; separate triage subagents write verdicts; valid fixes are
   applied one writer at a time; two rounds at most.
10. Use the `project-check` skill: `just check`, with `fix-quality` for any recipe that
    fails, then commit on the ticket branch with the hand-back notes quoting each output
    headed by the commit it ran at. Run `git fetch origin` (network, authorised by this
    step); if `origin/main` moved, merge it, rerun the Verification block and requote
    it. Pushing and opening a pull request are separately authorised: stop and ask.

## Acceptance criteria

- `pachinko` with no arguments writes `pachinko <version>` and exits 0;
  `pachinko serve --listen 127.0.0.1:0` writes `listening on 127.0.0.1:<port>`, answers
  `GET /healthz` with 200 `ok`, and exits 0 on SIGTERM and on SIGINT.
- The six routes in Goal answer with the shapes in step 6, and each of the six codes in
  Goal is produced by a wire test and asserted with its status by a unit test; the
  `status()` and `code()` matches have no wildcard arm.
- The owner applies every command in arrival order on one task, a `Pull` whose
  requester has gone is never applied, a full channel makes the sender wait and never
  answers a refusal, and a panicked owner makes `serve` return an error and the binary
  exit non-zero.
- `serve` refuses to start, with the reason on standard error, when the random source
  fails or the clock is before the epoch; a lease id on the wire is 32 hex digits and
  two ids never repeat.
- The six flags parse with their defaults; a negative, non-numeric or valueless flag is
  a usage error naming the flag; a zero `--max-consecutive-expiries` or
  `--max-pull-batch` is refused by the library's check.
- S019, S020, S028, S031 and S033 each have a wire test carrying its `// scenario:`
  comment, listed in the hand-back as exercised over HTTP, not owned.
- Each dependency in step 3 has its decision-0007 note, a full `x.y.z` version and a
  `{ workspace = true }` line; `just lock-check`, `just deny`, `just deps-unused` and
  `just effects` are green.
- `just coverage` reports at least 90% of lines over `crates/*/src/**`.
- `docs/reference/http-api.md` exists, is registered in `docs/manifest.yml` and reachable
  from `docs/README.md`, and every route, body and code it lists matches the code.
- `just check-docs` and `just check` are green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding
  is fixed or handed back.

## Verification

```sh
just lock-check
just fmt-check
just clippy
just effects
just test
just coverage
just doc
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

- The lease view (`GET /consumers/{consumer_id}/lease`) carries no `lease_id`, because
  T23's `LeaseView` is the pocket key, `expires_at` and the held records, following the
  `Consuming` surface's `exposes`; a consumer that lost its batch reply cannot learn the
  lease it must name for T25's actions. Adding the id to `LeaseView` is a one-field
  library change; the maintainer decides, and T25 is the natural place.
- A requester that disconnects between the owner's `is_closed` check and the grant holds
  a lease nobody will act on until it lapses (T26), `lease_ttl` later; the gap is stated
  in `http-api.md`. Closing it would need the handler to confirm the grant, which is a
  second round trip the specification does not have.
- The forced arm in each library-error conversion answers 503 `not_ready`; the maintainer
  may prefer a dedicated code for a library variant the service does not map, which the
  decision table rules out today ("no `internal` code").
- Until T27, a body over 2 MiB is 400 `bad_request`, not 413; until T26 no lease lapses;
  until T29 a stopped owner is the only 503 and the container how-to does not yet say
  how to run `serve`.
- `--listen 0.0.0.0:<port>` opens the surface to the network with no authentication;
  `security-model.md` says so, and whether the flag should warn is the maintainer's.
- The per-request log line is `debug`; T29's `--log-level` is how an operator turns it
  on.
