---
title: "Test and debug"
kind: "how-to"
audience: [contributor, maintainer, agent]
canonical_for: [test_workflow]
requires: []
---

# Test and debug

## Run the suites

```console
just test        # nextest over unit and integration tests, then the doctests
just test-doc    # the doctests alone: nextest cannot run them
just coverage    # nextest under instrumentation, with the coverage floor enforced
just clippy      # every lint the manifests declare, as errors
```

To iterate on one crate or one test, call nextest directly with `-p` and its filter
language, keeping the flags `just test` passes so the run sees the same features and
cannot touch the lockfile:

```console
INSTA_UPDATE=no cargo nextest run -p libpachinko --all-features --locked
INSTA_UPDATE=no cargo nextest run -p pachinko --all-features --locked -E 'test(run_writes)'
INSTA_UPDATE=no RUST_BACKTRACE=1 cargo nextest run -p libpachinko --all-features --locked -E 'test(name)'
```

The doctests run through cargo, not nextest, and narrow the same way:

```console
cargo test --doc -p libpachinko --all-features --locked
```

The tools live in `.pixi/envs/default/bin` and `.tools/bin`, which the `Justfile` puts on
`PATH` for every recipe and your shell does not. From a shell, prefix
`PATH="$PWD/.pixi/envs/default/bin:$PWD/.tools/bin:$PATH"`, or use
`pixi run --frozen cargo nextest ...`.

Framework configuration and conventions are described in
[Testing](../reference/testing.md); this page is about narrowing down a failure.

## Run the binary test

`crates/pachinko/tests/binary.rs` runs the built service as a user would. Cargo builds
the `pachinko` binary before it runs that crate's integration tests and hands the test
its path as `CARGO_BIN_EXE_pachinko`. The tests assert that the binary with no argument
exits zero, prints `pachinko`, a space, `libpachinko::version()` and one newline, and
writes nothing to standard error; that `serve --bogus 1` exits 1 with
`pachinko: usage: unknown flag --bogus`; and that `serve --listen 127.0.0.1:0` announces
its address, answers `GET /healthz` with `ok` there, and exits 0 on SIGTERM and on
SIGINT within a bounded wait. Run them alone with:

```console
INSTA_UPDATE=no cargo nextest run -p pachinko --all-features --locked --test binary
```

The version line has two tests: the unit test beside `run` in
`crates/pachinko/src/lib.rs` writes into a buffer, and the binary test reads the real
standard output. A change to the line fails both, and both are updated together, as
[Make your first change](../tutorials/first-change.md) walks through. When only the
binary test fails, the fault is between `run` and the process: `main.rs`, its exit
status, the signal handling, or something written to standard error. The wire tests in
`crates/pachinko/tests/http.rs` start the server in-process instead; run them with
`--test http` in place of `--test binary`.

## Run the service

Start the service on a free port; it prints the address it bound:

```console
$ cargo run -p pachinko --locked -- serve --listen 127.0.0.1:0
listening on 127.0.0.1:53817
```

In another terminal, with `ADDR` set to that address, join a publisher and a consumer,
publish `hello` (`aGVsbG8=` in base64), pull it, read the lease, and check health:

```console
$ ADDR=127.0.0.1:53817
$ curl -i -X POST "http://$ADDR/publishers/p1"
$ curl -i -X POST "http://$ADDR/consumers/c1"
$ curl -s -X PUT "http://$ADDR/pockets/tenant-a/records/k1" \
    -H 'content-type: application/json' \
    -d '{"publisher_id": "p1", "rank": 1, "payload": "aGVsbG8="}'
{"outcome":"created"}
$ curl -s -X POST "http://$ADDR/consumers/c1/pull" \
    -H 'content-type: application/json' -d '{"limit": 10}'
{"outcome":"batch","lease_id":"…","pocket_key":"tenant-a","expires_at":…,"records":[{"record_key":"k1","rank":1,"payload":"aGVsbG8="}]}
$ curl -s "http://$ADDR/consumers/c1/lease"
{"pocket_key":"tenant-a","expires_at":…,"records":[{"record_key":"k1","rank":1,"payload":"aGVsbG8="}]}
$ curl -s "http://$ADDR/healthz"
ok
```

The joins answer `201 Created` with no body. Press ctrl-c in the first terminal; the
service shuts down and exits 0. The lease actions, `POST /leases/{lease_id}/ack`,
`/nack`, `/renew` and `/release`, take the `lease_id` the pull returned. Every route,
body and error code is in [HTTP API](../reference/http-api.md); the request log is at
`debug`, which the default `info` level does not print.

## Narrow down a failing test

1. Run the single test first, by filter. A failure that only appears in the whole suite
   is usually shared state, and there is very little of it here: neither crate holds a
   global, and an effect reaches libpachinko only through a trait with a fake built per
   test (see [Architecture](../explanation/architecture.md)).
2. Read the assertion's left and right and, for a panic, the backtrace
   `RUST_BACKTRACE=1` prints. `ci.yml` and `pages.yml` set it for every job of theirs,
   so a test failure in CI already carries one.
3. If the expectation is about a rule, work it through by hand against the module under
   `docs/specs/` that states it. The specification is the arbiter, not the current code.
4. Do not weaken an assertion to make it pass. If the specification is wrong, change the
   specification — see [Work with the specifications](work-with-the-specs.md).

When `just check` fails, rerun the one recipe it named and read only its first failure;
[Quality gates](../reference/quality-gates.md) says what each recipe checks.

## Read and accept a changed snapshot

The snapshot recipes exist, and cargo-insta is pinned with the other tools, but no test
takes a snapshot yet: the `insta` crate becomes a development dependency with the first
snapshot. Until then `just snapshots-check` runs the tests and finds nothing to compare.
Once a test takes one:

1. Run `just test` or `just snapshots-check`. A mismatch prints the old and new text
   and names the snapshot. These recipes leave no pending file.
2. Read the diff and decide whether the interface change was intended. Keep the
   behaviour's asserting tests green; the snapshot establishes no correctness.
3. For an intended change, run `just snapshots-accept`. It checks all four locks, reruns
   all unit and integration tests with every feature and accepts pending snapshots
   without prompting. It also deletes a `.snap` file no test refers to, such as one a
   renamed or removed test left behind. It works for an agent without a terminal as well.
4. Read `git diff -- crates/` before committing and explain the change in the pull
   request or hand-back notes. Run `just snapshots-check` again: it also rejects a
   `.snap` file no test refers to.

If a run outside these recipes has left pending files, `just snapshots-review` opens
cargo-insta's interactive review at a terminal. Read each change there before accepting
it. The full rule and file locations belong to [Testing](../reference/testing.md#snapshots).

## Debug a coverage failure

`just coverage` prints the summary per file and fails below the floor. For the lines
themselves, render the HTML report, which lands at `target/llvm-cov/html/index.html`:

```console
cargo llvm-cov report --html
```

That reuses the profile data a `just coverage` run left behind; `--open` also opens the
report in a browser, and `cargo llvm-cov report --show-missing-lines` lists the missing
lines in the terminal instead. From cold,
`INSTA_UPDATE=no cargo llvm-cov nextest --workspace --all-features --locked --html`
does both at once.

The binary's lines count: cargo-llvm-cov instruments a binary an integration test runs,
so `crates/pachinko/src/main.rs` is covered by the binary test. The denominator is small
today, so one untested line moves the percentage by several points.
Two cases look alike and are not:

- **Untested behaviour.** Add the test. This is the common case.
- **Unreachable code.** A defensive branch no input can reach, such as a `match` arm for
  a value the types already rule out. Remove the branch, or make the types say so, rather
  than inventing a test that reaches it artificially.

Doctests are not measured, so a doc example does not count toward the floor; the lines
it exercises need a unit or integration test as well. Never lower `coverage_floor` in the
`Justfile`.

## Debug an effects or `--locked` failure

`just effects` runs `scripts/check-effects.sh`, which fails when libpachinko's normal
dependency tree names a banned crate: `rand`, `rand_core`, `getrandom`, `tokio`,
`async-std`, `smol`, `mio` or `socket2`. The line it prints names the crate; a
development dependency is exempt, so a test may still use one. It also fails when
libpachinko calls one of the standard library's clock, filesystem, environment, network,
process or thread APIs, which clippy reports as `use of a disallowed method` or `type`
with the path, from the bans in `crates/libpachinko/clippy.toml`. The bans hold in
libpachinko's own tests too, so a test that needs a real effect belongs under
`crates/pachinko/tests/`. Either way the fix is to take the effect through a trait
libpachinko defines, with its real implementation in `pachinko`, as
[Architecture](../explanation/architecture.md) and
[decision 0008](../decisions/0008-std-library.md) describe; never a suppression that
hides the call.

A `--locked` failure means `Cargo.lock` disagrees with a manifest: a dependency was edited
without relocking. Read [Maintain dependencies](maintain-dependencies.md) and relock
deliberately; never run `cargo update` from inside a fix for something else.

## Related pages

- [Testing](../reference/testing.md)
- [Quality gates](../reference/quality-gates.md)
- [Troubleshooting](../operations/troubleshooting.md)
- [Architecture](../explanation/architecture.md)
- [HTTP API](../reference/http-api.md)
