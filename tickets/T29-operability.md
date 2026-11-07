---
id: T29
title: "Operability: readiness, logging, timeouts, shutdown grace"
status: todo
depends_on: [T24]
parallel_with: [T25, T27]
branch: ticket/t29-operability
estimated_size: M
---

# T29: Operability: readiness, logging, timeouts, shutdown grace

## Context

T24 landed the service tracer: one owner task over the queue, the HTTP routes, the
exhaustive error map, SIGINT and SIGTERM installed before the address is announced, and a
graceful shutdown that waits for in-flight requests and then cuts the connections. It left
four operational gaps, each inherited from the HTTP prototype this wave grafts the service
onto, and this ticket closes them. The prototype's shortcomings, by line:

- The grace period is a constant, `GRACE` at PR 39 `crates/pachinko/src/serve.rs` line 30,
  ten seconds, and the drain at line 146 cuts the open connections when it runs out and
  then joins the remaining tasks "without a bound" (`join_logging`, line 167): a task that
  ignores the cut holds `serve` forever, and the prototype's own test at line 530 proves
  that it does.
- The log subscriber is initialised with no filter (lines 39 to 42) and the request
  middleware logs one `info` line per request (PR 39 `crates/pachinko/src/http.rs` line
  413, `log_request`; the line at 417). In the comparison that line cost over a third of
  the single-record publish rate (99,161 against 157,715 records/s with the log capped at
  WARN: the evidence table in T22's Context), so the default level must not pay for it.
- Nothing says whether the service is ready. `GET /healthz` (line 278) answers 200 as
  soon as the listener is bound, before the owner has taken a command and after it has
  stopped.
- No request can time out, and a connection that sends nothing holds its task until the
  process ends.

The decision table in T22's Context fixes the executor (one owner task, bounded commands,
oneshot replies) and the wire; this ticket adds nothing to either. It is a wave-2 lane
(`CONVENTIONS.md` §11): T25 and T27 edit the same service modules at the same time, so
every shared file below lists only this ticket's lines, `origin/main` is merged before the
push, and the verification is rerun and requoted at the merge commit.

The prototypes this ticket reads are heads of unmerged branches; a fresh worktree needs
`git fetch origin` (network, authorised by this step) before `git show` can read them.
Never check a prototype branch out into the ticket's worktree; read with
`git show <head>:<path>`.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 39, the HTTP service prototype | `tickets/T20-prototype-and-rest-api` | `cf1a6b2` | `git show cf1a6b2:<path>` |

## Goal

`pachinko serve` reports whether it is ready, logs at a level the operator chooses, bounds
every request and every idle connection, and stops within two grace periods the operator
chooses plus the application of the command or tick the owner was in when the second
elapsed, without aborting work before that bound. This ticket owns, with the test that
pins each and the hand-back that names it:

| Kind | Owned |
| --- | --- |
| Route | `GET /readyz`: 204 with no body while the owner is serving; 503 `not_ready` before the owner has begun and from the moment shutdown starts |
| Flags | `--log-level` (default `info`), `--request-timeout-ms` (default 30000), `--shutdown-grace-ms` (default 5000) |
| Codes | `not_ready` (503), `request_timeout` (408) |
| Behaviour | the per-request log line at `debug` only; the subscriber's filter from the flag, never from the environment; the connection cut at the grace and a bounded join after it; the owner told to stop; an owner that has stopped answering makes every route answer 503 `not_ready`, never 500 |

`just image` still builds, the container answers `/readyz`, and the handbook says how to
run and stop it.

## Non-goals

- No metrics endpoint, no structured or JSON log format, no log file: lines go to standard
  error as T24 left them.
- No change to `GET /healthz`: it stays T24's liveness route, 200 while the process serves
  HTTP.
- No `--sweep-ms` (T26), no request-shape limits (T27), no environment variable for any
  flag (`docs/reference/configuration.md`, "Environment": the service reads flags).
- No change to the `Dockerfile` or to the CI `image` job: the entrypoint stays the bare
  binary, which prints the version line the job asserts; `serve` and its flags are run
  arguments.
- No new kind of effect. The log writer, the signals and the timers are the service's,
  behind `tokio` and `tracing` as decision 0019 records; libpachinko is untouched.
- No change to the error map's shape: the two codes join T24's exhaustive `status()` and
  `code()` with no wildcard arm.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `crates/pachinko/src/config.rs` | shared, own lines | the three flags, their defaults and accessors; usage errors for an unknown level, a non-numeric value and a zero timeout |
| `crates/pachinko/src/serve.rs` | shared, own lines | the subscriber's level from the flag; the readiness watch; the grace from the flag; the cut, the stop and the bounded join |
| `crates/pachinko/src/actor.rs` | shared, own lines | the owner reports `Serving` as its first act and leaves its loop on stop; the handle's send to a gone owner is `NotReady` |
| `crates/pachinko/src/http.rs` | shared, own lines | `GET /readyz`; the timeout middleware; `not_ready` and `request_timeout` in `status()` and `code()`; the per-request line at `debug` |
| `crates/pachinko/src/lib.rs` | shared, own lines | the crate docs' operability paragraph |
| `crates/pachinko/src/error.rs` | shared, own lines | the `ServeError` variant for a shutdown overrun, if `Task(String)` does not carry it |
| `crates/pachinko/tests/http.rs` | shared, own lines | readiness and the timeout over the wire |
| `crates/pachinko/tests/binary.rs` | shared, own lines | the log level against `RUST_LOG`; SIGTERM clean and under a stalled connection |
| `Cargo.toml`, `crates/pachinko/Cargo.toml`, `Cargo.lock` | shared, own lines | only if step 6 adds a timeout crate: the decision-0007 note, `{ workspace = true }`, the lockfile from `just lock` |
| `docs/reference/http-api.md` | shared, own lines | the `GET /readyz` section; the two codes in the code table; what a 408 means for a publish; the shutdown window |
| `docs/reference/configuration.md` | shared, own lines | three rows in "Service flags"; `RUST_LOG` is ignored |
| `docs/operations/troubleshooting.md` | shared, own lines | four entries: not ready, request timeout, the port in use, the grace overrun |
| `docs/how-to/build-the-container-image.md` | shared, own lines | running `serve` in the container, the port, the flags, `docker stop` against the grace |
| `CHANGELOG.md` | shared, own lines | pachinko Unreleased, Added |
| `tickets/T29-operability.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree from `main` after T24 has merged (`README.md`, "How to pick up a
   ticket"), or, in a clean worktree already initialized on `main` whose branch carries
   nothing, create the branch there and skip `just initialize`. T25 and T27 may be in
   flight; neither is a dependency. Run `just check` on the untouched worktree and quote
   its last line as the baseline.
2. **The plan.** Use the `plan-change` skill. Read T24's hand-back notes and the modules
   it landed: where the owner loop takes a command, what `QueueHandle::send` returns when
   the receiver is gone, where `not_ready` sits in the error map (T24 pre-lands it as
   the one non-domain code; this ticket keeps it, owns it and pins it), and how `serve`'s
   scope drains. Read PR 39 `crates/pachinko/src/serve.rs`
   lines 101 to 184 (`scope`, `drain`, `join_logging`, `join_into`) and 199 to 283 (the
   `Connections` listener and the `Connection` wrapper whose reads fail once cut, line
   229). Write down the four behaviours, their tests and their pages before editing.
3. **The flags.** Use the `rust-change` skill. In `config.rs`, beside T24's flags (the
   shape PR 39 `crates/pachinko/src/config.rs` lines 125 to 131 reads): `--log-level`
   takes one of `error`, `warn`, `info`, `debug` and `trace`, default `info`, held as a
   `tracing::Level`; `--request-timeout-ms` takes milliseconds, default 30000, and 0 is a
   usage error as `--sweep-ms` 0 is, because it would time every request out;
   `--shutdown-grace-ms` takes milliseconds, default 5000, and 0 is legal: cut at once.
   An unknown level, a negative or non-numeric value and a flag without a value are usage
   errors with the flag's name in the message. Unit tests in the module (decision 0009):
   the defaults, each flag set, each refusal by message. A doctest on each accessor.
4. **Logging.** In `serve::run`, build the subscriber with the flag's level as its maximum
   (`tracing-subscriber` is pinned with `fmt` and `std` only, as PR 39's root `Cargo.toml`
   notes; no `env-filter` feature is added, so `RUST_LOG` and every other variable is
   ignored, and the page says so). The per-request line in `http.rs`: if T24 carried PR
   39's at `info`, move it to `debug`; if T24 has none, add it at `debug`. One line after
   the reply: method, path, status and elapsed milliseconds.
   Startup, the announced address, shutdown, the cut and T26's lapse counts stay `info`.
   Tests in `tests/binary.rs`: spawn `serve --listen 127.0.0.1:0` with `RUST_LOG=trace`
   set on the child through `Command::env` (never `std::env::set_var` in the test
   process), read the announced address, make one `GET /healthz`, send SIGTERM, and
   assert standard error carries no request line; the same with `--log-level debug`
   asserts exactly one. Each read of the child's output has a deadline that fails the
   test.
5. **Readiness.** A `watch` of a three-valued readiness, `Starting`, `Serving` and
   `Stopping`, created in `serve`'s scope and handed to the owner and the router. The
   owner sends `Serving` as the first act of its loop, before it takes any command;
   the scope sends `Stopping` the moment shutdown begins, before the grace wait, so a
   load balancer sees 503 while in-flight requests finish. `GET /readyz` answers 204 with
   no body on `Serving` and 503 `{"error": "not_ready", "message": <state>}` otherwise.
   A handler whose send to the owner fails because the owner is gone, or whose reply
   channel closes without a reply, answers the same 503 `not_ready`; there is no
   `internal` code and no 500 (the decision table's error map: exhaustive, no wildcard
   arm). Tests: a unit test in `http.rs` serves the router on `127.0.0.1:0` with a
   readiness watch the test holds, and asserts 503, then 204 after the test sends
   `Serving`, then 503 after `Stopping`, with a blocking client and a deadline on each
   read; a wire test in `tests/http.rs` against the real `serve` asserts 204 while
   serving, and 503 `not_ready` from a publish after the owner has been made to stop
   (the stop the scope uses, reached through whatever seam T24 left for its owner-panic
   test); an error-map test per new code asserts status and body, as T24's do.
6. **Timeouts.** Every route but `/healthz` and `/readyz` runs under a middleware that
   bounds the handler at `--request-timeout-ms` and answers 408
   `{"error": "request_timeout", "message": ...}` when it runs out. `tokio::time::timeout`
   inside an axum `from_fn` middleware needs no new crate; if the executing agent chooses
   a layer crate instead, it takes the decision-0007 note in the root `Cargo.toml` in the
   comment form the file already uses for divan, a full `x.y.z` caret version,
   `{ workspace = true }` in `crates/pachinko/Cargo.toml`, and `just lock` (network,
   authorised by this step). A command already sent to the owner is not withdrawn by the
   timeout: the owner applies it, and `http-api.md` says that a client whose publish
   timed out may find the record applied and should treat a retry's `stale`, or on the
   single `PUT` a 409 `equal_rank`, as that. The same bound is an idle timeout on each
   connection: in the `Connection` wrapper (PR 39 `crates/pachinko/src/serve.rs` line 229), a read that stays
   pending for the bound fails the connection, so a client that opens a connection and
   sends no request head cannot hold its task past the grace. Tests: a unit test in
   `http.rs` serves a test-only router with one route whose handler never completes under
   the middleware at 50 ms and asserts 408 with the body; a unit test in `serve.rs` opens
   a raw TCP connection, sends nothing, and asserts the server closes it within the bound
   plus a margin, failing on its own read deadline; a wire test in `tests/http.rs` is the
   positive control, a publish under the default bound answering 2xx.
7. **Shutdown grace and the bounded join.** `--shutdown-grace-ms` replaces the constant.
   The order, in `scope`: shutdown begins (a signal, a dropped `stop` or a failed task);
   readiness goes `Stopping`; the listener stops accepting and in-flight requests finish;
   when every task has ended, or the grace has elapsed, the open connections are cut and
   the owner is told to stop (its loop polls the same watch and returns; T26's ticker, if
   present, returns the same way); the `JoinSet` is then joined under a second bound equal
   to the grace, and only when that runs out are the remaining tasks aborted
   (`JoinSet::shutdown`) and `serve` returns `ServeError::Task` naming the overrun. An
   abort takes effect at the task's next yield point, and `shutdown` waits for each
   aborted task to reach one: the listener, the connections, the signal task and T26's
   ticker yield at every read, sleep and receive, and the owner yields between commands,
   applying each synchronously and finitely (T24: no owner borrow crosses an `.await`),
   so the overrun past the second bound is at most the application of the command or
   tick the owner was in when the second bound elapsed. The bound this ticket promises
   is therefore two grace periods plus that one application, and no multiple of the
   grace bounds the application: a command's work is finite but not bounded by the
   grace, because the queue is unbounded (open question 3), and a tick frees every due
   lease. Nothing is aborted before the second bound, and a task's own error inside the
   grace is still the one returned, as PR 39's drain ordered them. `exit_code` stays
   T24's: 0 on a clean
   stop, non-zero on any error. Tests: PR 39's `serve.rs` test at line 416 (a connection
   open past the grace is cut before `serve` returns) kept under the flag; a new test
   with a task that ignores the cut (a future parked at a yield point, so the abort
   lands at once) asserts `serve` returns `Err(ServeError::Task(_))` within three grace
   periods under `tokio::time::timeout`, where the timeout elapsing fails the test
   rather than passing it (the prototype's test at line 530 asserted the opposite); that
   test measures the cooperative case only, the two bounds and the scheduler, and the
   owner's overrun, the application it was in, is measured by no test, as the Open
   points record; T24's owner-panic test extended to assert `serve` returns within
   three grace periods, the owner being gone and nothing in application;
   `tests/binary.rs`: SIGTERM with no connection open exits 0 within the grace plus a
   margin (PR 39 `crates/pachinko/tests/binary.rs` line 36 is the shape), and SIGTERM
   with a stalled request open (a head promising a body, the body never sent, as PR 39
   `crates/pachinko/src/serve.rs` line 395 stalls one) under `--shutdown-grace-ms 100`
   exits non-zero within five seconds with the cut named on standard error.
8. **The image.** Run `just image` (network, authorised by this step: the build pulls the
   digest-pinned bases and fetches the crates inside the container), start the container
   with `serve --listen 0.0.0.0:8080` as run arguments and the port published, read
   `/readyz` from the host, stop it with `docker stop`, and quote the exit code
   `docker wait` prints. The `Dockerfile` and the CI `image` job are unchanged: the job's
   `docker run --rm pachinko:dev` still prints the version line and exits 0.
9. **The docs.** Use the `review-docs` skill. `http-api.md`: a `GET /readyz` section with
   the three states, the two codes in the code table, the 408 paragraph from step 6, and
   the shutdown window (503 from the first moment of shutdown; a client retries after the
   restart). `configuration.md`, "Service flags": the three rows, and one sentence that
   `RUST_LOG` is ignored. `troubleshooting.md`, one `##` per message: 503 `not_ready`
   (starting, stopping, or the owner has stopped: read standard error for the task error);
   408 `request_timeout` (the owner is behind; the command may still apply); `serve`
   failing with `binding the listener` and `Address already in use` (the port, `--listen`,
   port 0 for tests); `shutdown took longer than <n>ms` (a stalled client, the grace, the
   cut). `build-the-container-image.md`: replace the sentence that ports, health checks
   and configuration await the runtime decision with how to run the service in the
   container (`-p 8080:8080`, `serve --listen 0.0.0.0:8080` because the default loopback
   is unreachable from the host), `/readyz` as the check, and that `docker stop`'s
   timeout (`-t`, default ten seconds) must exceed twice `--shutdown-grace-ms`, the
   grace and the bounded join after it, plus the longest command or tick the owner
   applies, which the operator estimates from their own load, or the container is
   killed mid-join; with the default grace of 5000 ms the two grace periods alone
   already equal `docker stop`'s default, so the page says to pass `-t` above it or a
   smaller grace. `CHANGELOG.md`: pachinko Unreleased, Added. Run `just check-docs`.
10. **The review.** Run the `review-change` skill over the lane's diff: the ten
    invariants, `just effects` (libpachinko's tree is untouched), a test that asserts
    only that something was called, a public item without a doctest, a fact on a page
    that does not own its topic. Fix what it finds before the checkpoint below.

    **Adversarial review** (`CONVENTIONS.md` §14): the whole diff against the Goal table,
    Files touched and `CONVENTIONS.md` §11. Ask: whether the readiness tests see 503 in
    both the `Starting` and the `Stopping` state, or only 204; whether the grace tests
    pass when their timeout elapses; whether the log-level test could pass because the
    subscriber read `RUST_LOG`, or fail because another test's subscriber was global;
    whether a request can answer 500 by any path once the owner is gone; whether the
    idle-timeout test closes the connection itself; whether a task is aborted before the
    second bound; whether any line in a shared file belongs to T25 or T27; whether a page
    outside Files touched now says something false. Codex `gpt-6-astra` reviewers run
    read-only with fresh contexts, one lens each; separate triage subagents write
    verdicts; valid fixes are applied one writer at a time; two rounds at most. Then
    `just check` (`project-check` skill; a failing recipe goes to `fix-quality`) and
    commit on the ticket branch, the hand-back notes quoting each output headed by the
    commit it ran at.
11. **Merge `origin/main`.** `git fetch origin` (network, authorised by this step) and
    merge `origin/main`, which may now carry T25 or T27 (`CONVENTIONS.md` §11, the wave-2
    rule). Resolve conflicts in the shared modules by keeping both lanes' lines; never
    merge `Cargo.lock` by hand: on a lockfile conflict take either side and run
    `just lock` (network, authorised by this step). Rerun the Verification block and
    `just check`, requote them at the merge commit, commit. Then stop and ask: pushing
    and opening the pull request are separately authorised.

## Acceptance criteria

- `GET /readyz` answers 204 with no body while serving and 503 `not_ready` before the
  owner has begun and from the moment shutdown starts; the tests see both statuses.
- A route whose owner is gone answers 503 `not_ready`; `status()` and `code()` carry
  `not_ready` and `request_timeout` and still have no wildcard arm.
- With `--log-level` at its default and `RUST_LOG=trace` in the environment, a request
  writes no log line; with `--log-level debug` it writes one.
- A handler that overruns `--request-timeout-ms` answers 408 `request_timeout` with the
  JSON body; a connection that sends nothing is closed within the bound; `0` is refused
  as a usage error; the default is 30000.
- With `--shutdown-grace-ms` at its default, SIGTERM with no connection open exits 0; a
  connection open past the grace is cut; `serve` returns within two grace periods plus
  the application of the command or tick the owner was in when the second bound
  elapsed, even when a task ignores the cut, and the parked-future test shows the
  cooperative case within three grace periods; no task is aborted before the second
  bound.
- `just image` output is quoted; the container answers `/readyz` with 204 (the poll's
  `readyz poll exit 0` line) and exits 0 on `docker stop`; `docker run --rm pachinko:dev`
  still prints the version line.
- `docs/operations/troubleshooting.md` has the four entries; `configuration.md` has the
  three rows; `http-api.md` documents `/readyz`, both codes and the 408 retry paragraph.
- `just check-docs` and `just check` are green, requoted at the merge commit.
- The adversarial review in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just build
./target/debug/pachinko serve --log-level bogus; echo "exit $?"
./target/debug/pachinko serve --request-timeout-ms 0; echo "exit $?"
just test
just coverage
just image
docker run -d -p 127.0.0.1:18080:8080 --name pachinko-t29 pachinko:dev serve --listen 0.0.0.0:8080
# a bounded poll: 000 before the listener is bound and 503 before the owner serves are
# both expected on the way; the poll passes only if a 204 was seen before the deadline
(
  ready=1
  for i in $(seq 1 20); do
    code=$(curl -sS --max-time 2 -o /dev/null -w '%{http_code}' http://127.0.0.1:18080/readyz)
    echo "$code"
    if [ "$code" = 204 ]; then ready=0; break; fi
    sleep 1
  done
  docker stop -t 15 pachinko-t29; docker wait pachinko-t29; docker rm pachinko-t29
  exit "$ready"
); echo "readyz poll exit $?"
docker run --rm pachinko:dev; echo "exit $?"
just check-docs
just check
git status --short
```

`readyz poll exit 0` is the poll's only pass: `1` means no 204 arrived within twenty
attempts a second apart, each read bounded at two seconds. The subshell stops and removes
the container and then exits with the poll's status, so the figure printed is the block's
real exit status, not a variable echoed after a cleanup that succeeded, and the hand-back
quotes that line as it quotes any other exit status.

## Hand-back notes

Not started. The executing agent writes this section: each quoted output headed by the
commit it ran at, with its exit status (`CONVENTIONS.md` §11), and for each adversarial
checkpoint the harness and model, the findings, each verdict, what was applied and what
was handed back (`CONVENTIONS.md` §14).

## Open points

- The readiness route reports the owner's state only. Whether a service that has shed
  nothing but is far behind on its command channel should report not ready is a product
  question beside open question 3, not answered here.
- The idle timeout and the handler timeout share one flag. Separate flags wait until an
  operator asks for them.
- The grace period, its second bound, the longest command or tick the owner applies and
  `docker stop`'s ten seconds are four numbers the operator aligns by hand; the how-to
  says so, and nothing checks it. The owner's overrun past the second bound, that one
  application, is measured by no test in this ticket: the parked-future test measures
  the cooperative case, and a bound on the overrun would need a command whose duration
  the test controls, which this ticket does not add.
- The suspend-aware clock that 0019 lists as a follow-up is unaffected: a laptop
  sleeping through the grace is a clock question, not a shutdown one.
- A metrics endpoint and a structured log format are not planned by any ticket in the
  wave.
