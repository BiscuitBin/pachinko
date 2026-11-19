---
id: T38
title: "Slow-client spike: the unbounded connection, and existing ways to bound it"
status: todo
depends_on: [T33]
parallel_with: []
branch: ticket/t38-slow-client-spike
estimated_size: S
---

# T38: Slow-client spike: the unbounded connection, and existing ways to bound it

## Context

T29 and T33 grew a hand-rolled connection wrapper in `crates/pachinko/src/serve.rs`
(`Connection` and `Idle`: an idle read deadline, a first-byte head deadline and a
per-connection shutdown cut), because `axum::serve` gives no way to reach hyper's own
header-read timeout. Its edge cases (pipelined requests, `100 Continue`, heads hyper had
already buffered, 400 against 408) took four review rounds. On 2026-11-17 the maintainer
reversed that day's "Keep `Idle` as it is" call: the wrapper is removed, `listen` hands
axum the plain `TcpListener`, and this ticket follows. `tickets/T33-robustness.md`
holds the call under "Maintainer calls (2026-11-17)" and the reversal in the amendment
recorded after it.

This ticket is a spike. It names what a slow or silent client can do to the service now
that the wrapper is gone, looks for an existing way to bound each case, and ends with
findings and a recommendation the maintainer decides on. It writes no production code and
no new hand-rolled I/O wrapper. The decision is landed by the tickets it names.

Lines in the pinned crates are as of the `Cargo.lock` this ticket was written against:
axum 0.8.9, axum-core 0.5.6, hyper 1.12.0, hyper-util 0.1.21, tower 0.5.3, tokio 1.54.1
and socket2 0.6.5. A crate file is cited as the crate's name and a code-span path inside
it; after `just sync` the sources are in cargo's registry cache, at
`$CARGO_HOME/registry/src/*/<crate>-<version>/` (`~/.cargo` when `CARGO_HOME` is unset).
Step 2 re-checks the pins. This repository's own files are cited by item and heading, not
by line: T33's amendment had not merged when this ticket was written, so the executor
re-reads each at T33's merge.

### What the service does today

After the wrapper's removal:

- `listen` in `crates/pachinko/src/serve.rs` runs
  `axum::serve(listener, routes).with_graceful_shutdown(..)` and selects it against the
  cut. The cut ends `listen`'s wait; it closes no connection.
- `bounded` and `run_bounded` in `crates/pachinko/src/http.rs` wrap each handler in
  `tokio::time::timeout`, from the handler's start to the `Response` it returns, and
  answer 408 `request_timeout` past `--request-timeout-ms` (30000 by default).
  `/healthz` and `/readyz` sit outside the bound. The flag bounds nothing else
  (`docs/reference/http-api.md`, "Request timeout").
- Shutdown still returns at the grace: `serve` returns within two `--shutdown-grace-ms`
  plus the command or tick the owner was applying (the module doc of `serve.rs`).
- The workspace takes axum with `default-features = false` and the features `http1`,
  `json` and `tokio` (the root `Cargo.toml`). hyper, hyper-util and tower are not direct
  dependencies.

### What the pinned crates do

Each statement was read in the pinned source. None was run for this ticket; a statement
that describes behaviour on the wire is confirmed by a run in step 3, step 4 or, for
facts 7 to 9, step 5.

1. **One task per connection, no limit.** axum's serve loop accepts, then calls
   `handle_connection`, which spawns the connection with `tokio::spawn` (axum
   `src/serve/mod.rs` lines 284 to 294 and 353 to 416, the spawn at line 389). Nothing
   counts or caps the tasks.
2. **No timer, and no way to set one.** That task builds
   `hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())` and calls
   `serve_connection_with_upgrades` (axum `src/serve/mod.rs` lines 391 to 396). It sets
   no timer, and `axum::serve` takes no builder. Its doc says so: "intentionally simple
   and doesn't support any configuration. Use hyper or hyper-util if you need
   configuration" (lines 28 to 29).
3. **hyper's header-read timeout is inert without a timer.**
   `http1::Builder::header_read_timeout` defaults to 30 s and needs a `Timer` (hyper
   `src/server/conn/http1.rs` lines 253 and 361 to 373); `Builder::new` starts with no
   timer (line 246). `serve_connection` passes the default through `Time::check` (lines
   499 to 504), which, with no timer, logs `warn!` and returns `None` for a default and
   panics for a configured value (hyper `src/common/time.rs` lines 73 to 89).
4. **The dropped default is reported nowhere.** hyper's `warn!` expands to nothing
   without hyper's `tracing` feature (hyper `src/trace.rs` lines 109 to 116), and that
   feature also needs `--cfg hyper_unstable_tracing` (lines 5 to 11). The feature is
   `tracing = ["dep:tracing"]` (hyper `Cargo.toml` line 105), and `Cargo.lock`'s entry
   for hyper 1.12.0 lists no `tracing` dependency, so no crate in the workspace enables
   it. There is no `.cargo/config.toml` (decision 0007). Step 3 confirms it with one run.
5. **axum's own log lines are compiled out too.** axum's `tracing` feature is off
   (`Cargo.lock`'s entry for axum 0.8.9 lists no `tracing` dependency), so its `trace!`
   and `error!` expand to nothing (axum `src/macros.rs` lines 86 to 96). An accept error
   that is not a connection error, `EMFILE` included, makes the accept loop sleep one
   second and try again, and the `error!("accept error: {e}")` beside it logs nothing
   (axum `src/serve/listener.rs` lines 140 to 158).
6. **The header-read timeout is the only timeout in hyper's HTTP/1 server.** A search
   for `timeout` over hyper `src/server/conn/http1.rs` and `src/proto/h1/` finds only
   `header_read_timeout`, its documentation and its log line. Read that way, nothing
   there bounds the read of a body or the write of a reply.
7. **Where hyper's head timer runs.** It starts when `Conn::poll_read_head` first runs
   for a head and is cleared when the head has parsed (hyper `src/proto/h1/conn.rs`
   lines 216 to 240 and 279 to 283). When a connection goes idle after an exchange,
   `State::idle` asks for another read if a timeout is set, "so we can close the
   connection if another header isn't received in a timely manner" (lines 1132 to
   1162). Read that way, the one timeout would also bound a kept-alive connection
   waiting for its next request.
8. **hyper-util's version sniff sits in front of hyper.**
   `serve_connection_with_upgrades` always starts in `ReadVersion` (hyper-util
   `src/server/conn/auto/mod.rs` lines 264 to 285), which reads until the bytes stop
   matching the HTTP/2 preface and has no timer (lines 339 to 380). hyper's HTTP/1
   connection, and with it any header timer, is built only after that (lines 682 to
   688). `http1_only` "Does not do anything if used with
   `serve_connection_with_upgrades`" (lines 129 to 139). Read that way, a connection
   that has sent nothing is not yet under hyper's timer. hyper's own `http1::Builder`
   has no sniff (hyper `src/server/conn/http1.rs` line 470).
9. **What a timed-out head is answered.** The timeout returns
   `Error::new_header_timeout` (hyper `src/proto/h1/conn.rs` lines 262 to 271).
   `Server::on_error` builds an automatic reply only for a parse error, as 400, 431 or
   414 (hyper `src/proto/h1/role.rs` lines 493 to 511). Read that way, the connection
   is closed with no reply. T33's wrapper also closed a head that never completed with
   no reply. Its 408 answered a request whose head had arrived and whose body read
   failed at the deadline (`tickets/T33-robustness.md`, "Maintainer calls
   (2026-11-17)"), which hyper's head timer, cleared once the head has parsed (fact 7),
   never times.
10. **tower's limits and timeouts act on requests.** axum enables only tower's `util`
    feature (axum `Cargo.toml` lines 268 to 271); `limit` and `timeout` are separate
    features (tower `src/lib.rs` lines 175 to 176 and 194 to 195), and `limit` needs
    `tokio-util` (tower `Cargo.toml` lines 104 to 110), which `Cargo.lock` does not
    hold. `ConcurrencyLimit` "Enforces a limit on the concurrent number of requests the
    underlying service can handle": the permit is acquired in `poll_ready`, taken in
    `call` and held by the response future (tower `src/limit/concurrency/service.rs`
    lines 11 to 12 and 64 to 92, `src/limit/concurrency/future.rs` lines 17 to 22).
    `Timeout` starts its sleep in `call` (tower `src/timeout/mod.rs` lines 64 to 69).
    hyper calls the service once a head has parsed (hyper `src/proto/h1/dispatch.rs`
    lines 621 to 632), so neither sees a connection that has not sent one.
11. **What axum's listener module offers.** The `Listener` trait, its implementations
    for `TcpListener` and `UnixListener`, `ListenerExt::tap_io` and its return type
    `TapIo`, and nothing else (axum `src/serve/listener.rs` lines 9 to 24, 26 to 63, 66
    to 100 and 105 to 138). `tap_io` takes `FnMut(&mut Self::Io)`: it can set options on
    an accepted socket, and can neither replace the `Io` type nor refuse the connection.
12. **Socket options.** tokio's `TcpStream` has setters for `nodelay`, `quickack`,
    `linger` and `ttl`, and none for keepalive or a user timeout (tokio
    `src/net/tcp/stream.rs` lines 1193, 1268, 1335 and 1412). socket2 has
    `Socket::set_tcp_keepalive` (socket2 `src/socket.rs` line 2356) and
    `set_tcp_user_timeout`, the latter compiled only for Android, Fuchsia, Linux and
    Cygwin (socket2 `src/sys/unix.rs` lines 2543 to 2552), and `SockRef::from` borrows
    another crate's socket (socket2 `src/sockref.rs` line 81). socket2 is in
    `Cargo.lock` as tokio's dependency, not as the service's.
13. **What a hand-driven loop could reuse.** hyper-util has `rt::TokioTimer`
    (hyper-util `src/rt/tokio.rs` lines 103 and 292) and, behind its `server-graceful`
    feature, `server::graceful::GracefulShutdown` (hyper-util `src/server/mod.rs`,
    `src/server/graceful.rs` lines 20 to 72). axum enables hyper-util's `tokio`,
    `server` and `service` features, and `http1` through its own (axum `Cargo.toml`
    lines 97 to 101 and 190 to 197), not `server-graceful`.

### The concerns

Each is true of the service at T33's merge. Step 3 shows each with a scratch client, or
corrects it.

- **C1, a slow or never-finished request head (slowloris).** Nothing bounds it. A
  client that sends nothing, or sends a head a byte at a time, or never finishes it,
  holds its connection, its task and its descriptor until it closes the connection or
  the process exits (facts 2 to 4). This is T31's open point D-T29-6, which T33 carried
  and this ticket inherits; see Open points.
- **C2, an idle keep-alive connection that is never closed.** hyper keeps connections
  alive by default (hyper `src/server/conn/http1.rs` line 248), and nothing closes one
  that sends no further request.
- **C3, a slow body.** The `Json` extractor reads the body inside the handler, so a body
  that stalls is bounded by the handler's timeout and by nothing else: 30 s by default,
  then 408. `--max-body-bytes` bounds its size, not its time. Not checked: whether the
  connection is closed after that 408 or goes on waiting for the body.
- **C4, a client that never reads its reply.** The handler's timeout ends when the
  handler returns its `Response`. hyper writes the reply after that and has no write
  timeout (fact 6), so nothing bounds a write that waits on a client that has stopped
  reading. Not checked: how large a reply must be before its write waits at all.
- **C5, no cap on concurrent connections.** Every accepted connection gets a task
  (fact 1). When the process runs out of descriptors the accept loop sleeps a second,
  retries and logs nothing (fact 5); the connections already open are untouched. C1,
  C2 and C4 are each a way to hold one.
- **C6, connection tasks that outlive an embedded `pachinko::serve`.** The tasks are
  axum's, spawned outside the service's `JoinSet`; the cut and the abort end neither.
  The binary drops its runtime when `serve` returns, which closes them. Under a
  caller's runtime one can outlive `serve`; a request that completes on it then is
  still answered, and a command it sends is refused 503 `not_ready`, because the owner
  has stopped (`serve`'s `# Errors` in `crates/pachinko/src/serve.rs`).
- **C7, one flag or several.** `--request-timeout-ms` bounds a handler today. Whatever
  is adopted needs its bound configured: under the same flag, under one new flag, or
  under one per bound. The maintainer's "Keep one flag" call of 2026-11-17 was made for
  the wrapper; T31's D-T29-2 is the same question.

### The two questions nobody has run

- **Q1, what graceful shutdown does to a connection that holds a partial request
  head.** The answer decides whether the cut is still needed and what a stalled client
  costs a shutdown. The source suggests it depends on the connection's state, so the
  spike runs each state and records what the client sees and when axum's wait ends:
  - *Nothing sent, or only bytes that still match the HTTP/2 preface* (a lone `P`, the
    first byte of `PUT` and `POST`, is one). The connection is still in `ReadVersion`;
    `graceful_shutdown` cancels it, and the next poll returns an `Interrupted` error
    (hyper-util `src/server/conn/auto/mod.rs` lines 333 to 349 and 621 to 623).
  - *Part of the first head.* hyper-util hands the shutdown to hyper (lines 624 to
    625; hyper `src/server/conn/http1.rs` lines 139 to 141 and 547 to 553).
    `Dispatcher::disable_keep_alive` closes the connection only if it is idle or has
    read and written nothing (hyper `src/proto/h1/dispatch.rs` lines 91 to 100). A new
    connection starts busy, not idle (hyper `src/proto/h1/conn.rs` lines 57 and 903 to
    911), and a partial head sits in the read buffer (lines 200 to 204; hyper
    `src/proto/h1/io.rs` lines 174 to 229).
  - *Part of a later head on a reused connection.* After a whole exchange the state is
    idle, and stays idle until a whole head has parsed (hyper `src/proto/h1/conn.rs`
    lines 299 and 1132 to 1144), and an idle connection is closed (lines 903 to 906 and
    1077 to 1082).
  - *A reused connection with nothing buffered*, and *a whole head with its body
    stalled*, as controls.
- **Q2, whether hyper's dropped default `header_read_timeout` is reported at run
  time.** Read from the source, it is not (fact 4). The spike confirms it: the service
  at `--log-level trace` logs its own per-request line for a request (the positive
  control, `log_request` in `crates/pachinko/src/http.rs`) and no line naming
  `header_read_timeout`.

### The candidates

No candidate is chosen. For each, the spike reads the pinned source, prototypes under
`ai_tmp/` where reading does not settle it, and fills one row of the matrix below.

| Candidate | What it is | What must be checked |
| --- | --- | --- |
| A, hyper's `header_read_timeout` with a `Timer` | A hand-driven accept loop over hyper-util's builder, or hyper's `http1::Builder`, in place of `axum::serve`. hyper and hyper-util become direct dependencies, each with a decision-0007 note (`docs/decisions/0007-dependency-policy.md`). | Which of C1 and C2 it bounds on the wire (facts 7 and 8), and whether a silent new connection escapes it; what the client sees at the timeout (fact 9), against T33's wrapper, which closed a head that never completed with no reply; Q1 under the timer; what the loop must take over from `axum::serve` (accept errors, graceful shutdown, the wait for open connections) and what hyper-util supplies (fact 13); whether a loop that owns its tasks settles C6 and can cap C5; the features and any new crate in `Cargo.lock`; the service code it adds, in lines. |
| B, `axum-server` | A crate not in `Cargo.lock` or the registry cache. This ticket claims nothing about it. | Against one pinned version: what timeouts and limits it exposes and which concerns they bound; whether it works with axum 0.8.9 and hyper 1.12.0; its graceful shutdown and whether it bounds C6; its dependency tree, licence and advisories under `deny.toml`; its maintenance. |
| C, axum's listener extensions | `Listener`, `ListenerExt::tap_io` and `TapIo` (fact 11). | Whether a `Listener` can cap connections (C5) without an `Io` wrapper, given that a permit must live as long as its connection; what `tap_io` can usefully set (candidate F); whether an axum release after 0.8.9 adds anything (network, step 6). |
| D, `tower-http` timeout layers | A crate not in `Cargo.lock` or the registry cache; axum names it only as an optional dependency behind its private docs feature and as a development dependency (axum `Cargo.toml` lines 75 to 80, 273 to 304 and 380). This ticket claims nothing about what it does. | Against one pinned version: which layers exist and what each bounds (the request, the request body, the reply body); whether any reaches C3 or C4 better than `bounded` does; that none can reach C1, C2 or C5, since a layer runs after a head has parsed (fact 10); whether one would replace `bounded`; the new crates. |
| E, tower concurrency limits | `ConcurrencyLimit` and its relatives behind tower's `limit` feature (fact 10). | That it caps requests in flight, not connections; whether placing it on the make-service caps connections, given that axum awaits `make_service.ready()` and `call` for each accepted connection before it spawns (axum `src/serve/mod.rs` lines 371 to 382) and the permit's lifetime there; tower as a direct dependency and `tokio-util` as a new crate; whether a cap that refuses, rather than delays, fits decision 0019's "never a capacity refusal" and the specification's `NeverRefusedForCapacity`. |
| F, socket options | TCP keepalive and `TCP_USER_TIMEOUT` on accepted sockets (fact 12). | What each option bounds, from the operating system's documentation, and which concerns that reaches (a dead peer is not a slow one); the platforms (the image is Linux, development is macOS); how to set them: `tap_io` with socket2's `SockRef`, or on the listening socket if accepted sockets inherit; socket2 as a direct dependency with its note; that no `unsafe` is needed (invariant 4). |
| G, a fronting proxy as the documented answer | No change to the service; the handbook says what must stand in front of an exposed listener, as `docs/explanation/security-model.md` ("Request-shape limits") begins to. | For two or three named proxies, from their documentation: the default and configurable bound for each of C1 to C5; what the handbook would have to state, and on which owning page; what stays open for a client that reaches the service without the proxy; that it does nothing for C6. |
| H, an upstream axum change | A timer, a header timeout or a connection limit on `axum::serve` itself. | From axum's changelog and issue tracker (network, step 6): whether a release after 0.8.9 has it, in which case the answer may be a version bump; whether an issue or pull request is open, and its state. Nothing is filed by this ticket. |

The matrix the hand-back fills, one cell per candidate and concern, each `covers`,
`partly`, `no` or `unknown`, with its evidence (a crate path and lines, or a quoted run)
beside it. `unknown` needs its reason.

| Candidate | C1 | C2 | C3 | C4 | C5 | C6 | Cost |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Today | no | no | handler timeout | no | no | no | none |
| A to H | | | | | | | |

### Risks (`CONVENTIONS.md` §13)

- **A bound shown with no control.** A prototype that closes a slow connection proves
  nothing unless the same client, against the service as it is, is left open. Every
  bound the hand-back quotes follows its control (§11, "Checks fail closed").
- **Timing on a loaded host.** A bound is quoted as a window, with the timeout it was
  run under, never as an instant.
- **The network at first run, none in `check`.** Candidates B and D, and the reading
  for G and H, need the network; steps 5 and 6 authorise it. A scratch crate lives
  under `ai_tmp/` with its own lockfile and never touches the workspace's manifests or
  `Cargo.lock`.
- **lychee and typos over `tickets/`.** The hand-back writes paths and URLs as code
  spans, never as links.

## Goal

The hand-back holds, for the service at T33's merge: each of C1 to C6 shown by a run;
Q1 answered for each state and Q2 confirmed; one findings entry for each of candidates
A to H, with its pinned version, its evidence and its cost; the matrix filled; and a
recommendation that says, for each concern, adopt a named candidate, document it, or
leave it open, with the flags it would add (C7), each new dependency with a draft
decision-0007 note, and whether a decision record is needed and which record it revises.
The maintainer's decision on that recommendation is recorded, quoted and dated, and the
tickets that would land it are named. Nothing outside `tickets/` changes.

## Non-goals

- No production code: nothing under `crates/`, no manifest, no `Cargo.lock`, no
  `Justfile` recipe, no page under `docs/` and no specification changes. Prototypes and
  scratch clients stay in `ai_tmp/` and are never committed.
- No new hand-rolled I/O wrapper, as a prototype or as a recommendation. This ticket
  reads that as a type of the service's own that implements the socket's read and write
  traits around a connection, which is what `Connection` and `Idle` were. Whether a
  hand-driven accept loop (candidate A) or a custom `Listener` (candidate C) is on the
  allowed side of that line is the maintainer's to say; see Open points.
- No decision record on this branch. If the decision needs one, it is handed back as
  its own pull request to `main`, because a new record under `docs/decisions/` is
  outside Files touched.
- No issue, pull request or comment filed upstream. A draft stays in `ai_tmp/`.
- No new flag and no changed default. C7 ends in a recommendation.
- No change to the shutdown's design: the grace, the cut and the abort stay as T33
  left them, and Q1's answer is a finding.
- No answer to T31's other open points, and nothing on TLS, authentication, rate
  limiting or HTTP/2.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tickets/README.md` | shared, own lines | T38's Status cell |
| `tickets/T38-slow-client-spike.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree from `main` after T33 has merged (`README.md`, "How to pick up a
   ticket") and run `just initialize` there (network, authorised by this step). Run
   `just check` on the untouched worktree and quote its last line as the baseline.
2. **The re-read.** Use the `plan-change` skill. Confirm the seven pins in Context
   against `Cargo.lock`; if one has moved, re-locate every line this ticket cites in
   that crate by the item it names, and record the new lines. Re-read the module doc of
   `crates/pachinko/src/serve.rs`, `listen`, `serve`'s `# Errors`, `bounded` and
   `run_bounded` in `crates/pachinko/src/http.rs`, and "Request timeout" and "Shutdown"
   in `docs/reference/http-api.md`. Record every sentence of "What the service does
   today" and "The concerns" that no longer matches, and work from what the tree says.
3. **Today's service, observed.** Run `just build` and start `target/debug/pachinko
   serve --listen 127.0.0.1:0`, reading the port from its `listening on <address>`
   line. With a scratch client under `ai_tmp/t38/` over a plain TCP socket (std's
   `TcpStream`, as the socket tests in `crates/pachinko/src/serve.rs` use), show each
   of C1 to C5 against it under a short `--request-timeout-ms`: the connection that is
   still open past the bound, the 408 for the stalled body and what the connection
   does after it, and the accept loop under a lowered `ulimit -n` in the shell that
   starts the service. For C6, a scratch crate under `ai_tmp/t38/` that awaits
   `pachinko::serve` on its own runtime and shows a connection answered after `serve`
   has returned. For Q2, the same service at `--log-level trace`, its standard error
   captured: one request first, so that its per-request line shows the writer works at
   that level, then the search for `header_read_timeout`. These runs are the controls
   every later bound is compared with.
4. **Q1.** With the scratch client, run each state Context lists against the service
   and send SIGTERM: record what the client reads, when its connection closes, whether
   `serve` returned at the first grace or the second, and its exit status. Where a run
   disagrees with Context's reading of the source, the run stands and the hand-back
   says where the reading went wrong.
5. **The candidates in the lock, and the two that are not.** For A, C, E and F, read
   the pinned source and prototype in a scratch crate under `ai_tmp/t38/`, whose
   manifest carries its own empty `[workspace]` table and its own lockfile, built
   offline from the registry cache. For B and D: **network, authorised by this step**,
   to fetch one exact version of each into such a scratch crate; record the version
   and its checksum, and cite every claim by a path and lines in that version. Never
   add a crate to the workspace's manifests, and never run `just lock`. Each prototype
   is run against the same scratch client as step 3, and each bound it shows is quoted
   after its control. Fill the candidate's findings entry and its matrix row.
6. **The proxy and upstream.** **Network, authorised by this step**, read-only: for G,
   the timeout and connection-limit documentation of two or three proxies the
   maintainer would plausibly run; for H and C, axum's changelog and issue tracker
   past 0.8.9. Quote each source by its URL, as a code span, with the date read.
   Nothing is filed or posted.
7. **The findings and the recommendation.** Write the hand-back: the observed
   baseline, Q1 and Q2, the eight findings entries, the matrix, and the recommendation
   in the shape the Goal gives. For C7, name each flag the recommendation adds, its
   default and what it bounds. For each new direct dependency, draft its decision-0007
   note. Say whether the recommendation revises decision 0019, and name the tickets
   that would land it.

   **Adversarial review** (`CONVENTIONS.md` §14), over the hand-back notes and the
   prototypes under `ai_tmp/t38/`, before the maintainer is asked. Ask: whether any
   statement about a crate lacks a path and lines in a pinned version, or a quoted run;
   whether any claim about `axum-server` or `tower-http` comes from memory; whether a
   `covers` cell rests on reading alone where the behaviour is a matter of timing;
   whether any bound is quoted without its control, or as an instant; whether Q1 is
   answered for every state, and by a run; whether a concern was dropped or merged
   into another; whether the recommendation, or a prototype it rests on, is a
   hand-rolled I/O wrapper under another name; whether a stated cost leaves out a
   feature, a transitive crate or a dependency note; and whether anything outside
   Files touched changed. Two reviewer subagents with fresh contexts run in parallel,
   one lens each, `conformance` and `breakage`, each running the §14 harness the
   executing agent is not: from Claude Code, `codex exec -m gpt-6-astra -s read-only
   -C <worktree> -o ai_tmp/review/T38-1/findings-<lens>.md "<brief>" < /dev/null`
   (`-o` writes only the reviewer's last message, and a read-only reviewer writes no
   file of its own); from Codex, `claude -p --model claude-opus-5-5 --permission-mode
   plan "<brief>" > ai_tmp/review/T38-1/findings-<lens>.md`; separate triage subagents
   check every finding against the tree and the pinned sources and write
   `ai_tmp/review/T38-1/verdicts.md`, each finding valid with its fix or invalid with
   the reason; one writer applies the valid fixes one at a time, only within Files
   touched; if a high-severity finding was valid, a second round reviews the fix diff
   and writes beside the first with a `-2` suffix; two rounds at most. A finding that
   a candidate should have been chosen differently goes to the maintainer with the
   recommendation, never into a fix.
8. **Authorisation required: the maintainer's decision.** Give the maintainer the
   recommendation, the matrix and the review's open findings, and ask, for each of C1
   to C6, adopt, document or leave open; for C7, the flags; where the line on a
   hand-rolled wrapper falls for candidates A and C; whether hyper's timeout closing a
   head that has not completed with no reply, as T33's wrapper did, is acceptable, if
   candidate A is in the recommendation; and whether a decision record is wanted. Then stop and do
   nothing further until the maintainer answers. Quote the answers in the hand-back,
   dated. An answer that names something outside the candidates is a new design:
   record it and stop again. Write nothing that implements the answer; name the
   tickets and the decision-record pull request that would, in Open points.
9. **Merge and gate.** Use the `project-check` skill: `just check`, with `fix-quality`
   for any failing recipe. Run `git fetch origin` (network, authorised by this step)
   and merge `origin/main` (`CONVENTIONS.md` §11). Rerun the Verification block at the
   merge commit and requote it. Set `status: done` and T38's Status cell in
   `tickets/README.md` to `done`, and commit on the ticket branch with the hand-back
   notes, each quoted output headed by the commit it ran at. No review runs before
   this commit: since T38-1 only the maintainer's quoted answers and the status have
   changed. **Authorisation required: push and open a pull request.** Stop and ask.

## Acceptance criteria

- The baseline `just check` line is quoted, and step 2's record says which pins and
  which sentences of Context held.
- Each of C1 to C6 is shown against the service at T33's merge by a quoted run, or its
  statement in Context is corrected with the run that contradicts it.
- Q1 is answered for each state Context lists, by a run; Q2 is confirmed by a run.
- Each of candidates A to H has a findings entry with its pinned version, its evidence
  and its cost, and a matrix row with no empty cell. No claim about `axum-server` or
  `tower-http` lacks a version, a path and lines.
- Every bound a prototype shows is quoted after its control, as a window, with the
  timeout it ran under.
- The recommendation gives one outcome for each of C1 to C6, the flags for C7, a draft
  decision-0007 note for each new direct dependency, and says whether a decision record
  is needed and which record it revises.
- The maintainer's decision is quoted in the hand-back with its date, and the tickets
  and the pull request that would land it are named in Open points.
- The adversarial review in Steps ran (`CONVENTIONS.md` §14), and every valid finding
  is fixed or handed back.
- `git diff --name-only origin/main...HEAD` lists `tickets/README.md` and
  `tickets/T38-slow-client-spike.md` and nothing else.
- `just check` is green at the commit that merges `origin/main`, and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
line that must print nothing follows its positive control.

```bash
set -o pipefail
# the pins Context cites
rg -n -A1 '^name = "(axum|axum-core|hyper|hyper-util|tower|tokio|socket2)"$' Cargo.lock; echo "exit $?"
# positive control: the block filter finds hyper's and axum's entries
awk 'BEGIN{RS=""} /\nname = "(hyper|axum)"\n/' Cargo.lock | rg -c '^name = '; echo "exit $? (must print 2)"
# neither entry names a tracing dependency, so neither crate's log macros are compiled in
awk 'BEGIN{RS=""} /\nname = "(hyper|axum)"\n/' Cargo.lock | rg -n '"tracing"'; echo "exit $? (must be 1)"
# positive control, then the crates this ticket says the lock does not hold
rg -n '^name = "tracing"$' Cargo.lock; echo "exit $? (positive control, must match)"
rg -n '^name = "(tower-http|axum-server|tokio-util)"$' Cargo.lock; echo "exit $? (must be 1)"
# scratch work is ignored
git check-ignore -v ai_tmp/t38; echo "exit $? (must be 0)"
# positive control: the branch changed its two files; then nothing else
git diff --name-only origin/main...HEAD; echo "exit $? (must list the two tickets/ paths only)"
git diff --stat origin/main...HEAD -- crates docs Cargo.toml Cargo.lock Justfile; echo "exit $? (must print nothing)"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Hand-back notes

### Baseline

### The pins and the re-read (step 2)

### Today's service, observed (step 3)

### Q1: graceful shutdown and a partial head (step 4)

### The candidates (steps 5 and 6)

### The matrix and the recommendation (step 7)

### T38-1: the findings (step 7)

### The decision (step 8)

### Step 9: merge and gate

## Open points

- **D-T29-6, inherited from T33.** T31 recorded it as "A client trickling one byte per
  bound keeps a connection open until the shutdown's cut; `security-model.md`
  documents it (D-T29-6). Owner: maintainer." (`tickets/T31-mvp-integration.md`, "Open
  points"). T33's first-byte deadline closed it, and the wrapper's removal reopened it
  wider: nothing bounds a head at all, and the cut no longer closes the connection.
  It is C1 here. Owner: this ticket until step 8, then the ticket the decision names.
- **D-T29-2, one flag.** T31 recorded "The idle timeout and the handler timeout share
  one flag (D-T29-2, low)", and the maintainer's "Keep one flag" call of 2026-11-17
  answered it for the wrapper. With the wrapper gone it is C7, asked again at step 8.
- Where "hand-rolled I/O wrapper" stops: whether a hand-driven accept loop over
  hyper-util (candidate A), or a `Listener` of the service's own (candidate C), is
  allowed. The spike evaluates both and asks at step 8. Owner: maintainer.
- hyper's header timeout, read from the source, closes a head that has not completed
  with no reply, as T33's wrapper did. The 408 the maintainer called for in T33 was for
  a body read that failed after a whole head, which hyper's head timer does not time.
  Step 5 checks it on the wire and step 8 asks whether it is acceptable. Owner:
  maintainer.
- Whether the decision needs a record, and whether it revises decision 0019, is asked
  at step 8; the record is its own pull request to `main` (Non-goals).
- `crates/pachinko/src/serve.rs` and `docs/explanation/security-model.md` name T38 as
  the owner of these gaps. Rewording them once the decision is made belongs to the
  ticket that lands it, not to this one.
