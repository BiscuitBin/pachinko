---
id: T30
title: "Performance evidence, never gated"
status: todo
depends_on: [T26, T27]
parallel_with: [T28]
branch: ticket/t30-performance-evidence
estimated_size: M
---

# T30: Performance evidence, never gated

## Context

Decision 0019 chose the MVP's runtime, wire and store on measurements of the prototypes
(T22's Context carries the decision table and the evidence table). Two of its numbers
were never measured on the thing it chose: the HTTP batch publish rate, which is the
reopen trigger for Arrow Flight, and the lapse lag of a tick lane the owner polls first.
This ticket measures the MVP as built by T23 to T27, in process and over loopback, and
records the numbers in 0019 beside the prototype figures, with the machine and harness
named, so that the reopen triggers can be read against evidence rather than against a
prototype that no longer exists. Nothing here gates: timings measure the machine, so
`just bench` and `just loadgen` stay outside `just check` and CI (decision 0017).

The figures this ticket measures against, from T22's evidence table. Machine and harness,
named once: one Apple silicon Mac (macOS, Darwin 25.4.0, 18 CPUs), rustc 1.99.1,
measured 2026-10-31 and 2026-11-03; end-to-end runs over loopback with closed-loop
clients; medians of 5 runs; 64-byte payloads; P = 1,000 pockets. The in-process numbers
are single-thread over the prototype's own `Queue` with its fake clock.

| Measurement | Prototype figure |
| --- | --- |
| In process, W1 publish at P=1,000 | 287,895 ops/s (`ArrowRecords`) vs 4,246,025 ops/s (`MemoryRecords`), 14.7x |
| In process, W2 supersede | 781,874 vs 6,118,040 ops/s (7.8x) |
| In process, W3 pull 16, ack, release | 460,431 vs 2,550,028 ops/s (5.5x) |
| In process, W4 publish with every fifth op a consume step | 301,528 vs 2,831,593 ops/s (9.4x) |
| In process, W5 pull then lapse on tick | 257,835 vs 801,351 ops/s (3.1x) |
| Single-record publish, C=64 | PR 39 99,161 records/s as shipped (INFO log), 157,715 with the log capped at WARN; PR 38 76,058 (`ArrowRecords`), 77,920 (`MemoryRecords`) |
| Batch publish, C=8, 256 rows per batch | PR 38 121,394 rows/s as shipped; 1,565,592 rows/s with `MemoryRecords`, the Flight reopen figure |
| Batch publish, C=64, 1,024 rows per batch | PR 38 139,374 rows/s as shipped; 2,100,471 rows/s with `MemoryRecords` |
| Lapse lag p50 under batch load | PR 38 as shipped (`Tick` in the client FIFO) 2,038 ms (max 3,257 ms), re-run 2,411 ms; PR 38 with `MemoryRecords` 439 ms; T21 560 ms; 315 to 689 ms in the other probe runs |
| RSS, 10 s of batch publishing, PR 38 with `MemoryRecords` | 8,191 MiB; about 0.45 KiB per record at consume C=64 (0.93 KiB with `ArrowRecords`) |

Caveats 0019 already states and this ticket repeats beside its own numbers: one machine,
nothing on Linux; closed-loop clients understate tail latency; the `MemoryRecords` and
WARN-log variants were scratch one-change variants neither prototype shipped; the
prototypes' unit of an in-process "op" is not recorded in the repository, so 0019 states
this ticket's unit beside each figure and compares with that caveat. The MVP's default
log level has no per-request lines (T29), so its single-record publish figure is read
against both PR 39 figures. The reopen trigger for Arrow Flight is a ratio of the
1,565,592 rows/s figure that the maintainer sets; this ticket reports the MVP's rate and
the quotient, and decides nothing.

The prototype this ticket reads is the head of an unmerged branch; a fresh worktree needs
`git fetch origin` (network, authorised by step 1) before `git show` can read it. Never
check a prototype branch out into the ticket's worktree; read with `git show <head>:<path>`.

| Source | Branch | Head | Read with |
| --- | --- | --- | --- |
| PR 38, the Flight service prototype | `tickets/t20-pachinko-prototype-complete` | `cb77f07` | `git show cb77f07:<path>` |

The file to read: PR 38 `crates/pachinko/benches/arrow.rs`, a divan bench over the
prototype's `Queue` on its fake clock (`drain` line 84, the `publish` bench line 101),
the shape W1 to W5 follow. The excluded-crate pattern is this repository's own:
`fuzz/Cargo.toml`, the `fuzz/` lines of the `Justfile`, and `tickets/T16-bench-and-fuzz.md`
for how the pattern landed and what it rippled through the handbook. The load generator
is written fresh in this ticket; the maintainer may supply a reference generator from the
comparison session, which is not in the repository. If supplied, it is read for the
workload definitions and the measurement method only, and never copied in: its figures
are the prototypes', not the MVP's.

## Goal

`just bench` runs five divan workloads, W1 to W5, over the library's queue at 10, 1,000
and 100,000 pockets and prints ops per second; an excluded crate `loadgen/` on the `fuzz/`
pattern builds a closed-loop load generator and lapse probe over HTTP, run by
`just loadgen` outside `check`; the publish, batch publish, consume, mix and lapse-lag
workloads are measured against a running `pachinko serve` on loopback; decision 0019
gains an "Evidence from the MVP" section with every number, the machine and harness, the
service flags, the unit of each figure and the quotient against the Flight reopen figure;
`just check` is green with the fourth lockfile in place.

## Non-goals

- No gate and no threshold: no recipe here enters `pyproject.toml`'s
  `[tool.biscuit-games-tooling] recipes`, no CI job runs a bench or the load generator,
  and no number fails a build.
- No decision: the Flight reopen ratio is the maintainer's; 0019 gains evidence, not a
  verdict.
- No async in `loadgen/`: blocking `ureq` clients on std threads, closed loop. No
  dependency on either workspace crate; it speaks HTTP only.
- No open-loop or latency-distribution harness; closed-loop medians are the comparison
  the prototypes were measured by, and the caveat stays.
- No RSS bound and no answer to open question 3; an RSS reading, if taken, is reported.
- No change to the library's public API for a bench; a bench drives the queue through
  what T23 to T26 published and the `testing` feature.
- No Linux measurement; the caveat stays and is listed in Open points.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `Cargo.toml` | shared, own lines | `exclude = ["fuzz", "loadgen"]` |
| `crates/libpachinko/Cargo.toml` | shared, own lines | The `[[bench]] name = "workloads"` table with `harness = false` |
| `crates/libpachinko/benches/workloads.rs` | new | W1 to W5 over `Queue<MemoryRecords, FakeClock, CountingLeaseIds>` |
| `loadgen/Cargo.toml` | new | The excluded crate `pachinko-loadgen`: `publish = false`, `rust-version`, the two dependencies with decision-0007 notes, the lint tables copied from `fuzz/Cargo.toml` |
| `loadgen/Cargo.lock` | new | Committed; the pin for the crate's separate graph |
| `loadgen/src/main.rs` | new | The entry point: argument parsing, the subcommand dispatch, the exit status |
| `loadgen/src/client.rs` | new | One `ureq` agent per client thread; every reply read whole and parsed; an error is counted, never ignored |
| `loadgen/src/workloads.rs` | new | `publish`, `batch`, `consume`, `mix` |
| `loadgen/src/probe.rs` | new | The lapse probe |
| `loadgen/src/report.rs` | new | One line per run, the median line, the error count |
| `Justfile` | shared, own lines | The `loadgen` recipe, and a `--manifest-path loadgen/Cargo.toml` line beside each `fuzz/` line in `sync`, `lock`, `lock-upgrade`, `lock-check`, `fix`, `fmt-check` and `clippy` |
| `.pre-commit-config.yaml` | shared, own lines | The `fmt-check` hook's `files:` widened to `loadgen/` |
| `.gitignore` | shared, own lines | `/loadgen/target/` |
| `.dockerignore` | shared, own lines | `loadgen/target/` |
| `renovate.json` | shared, own lines | The `rust-version` regex manager's `managerFilePatterns` gains `loadgen/Cargo.toml`; its description counts four places |
| `docs/decisions/0019-service-runtime-tokio-and-http.md` | shared, own lines | The "Evidence from the MVP" section |
| `docs/decisions/0007-dependency-policy.md` | shared, own lines | One paragraph under "Harness dependencies": the `loadgen/` crate, its two dependencies, its lockfile, and that it names no workspace crate |
| `docs/reference/testing.md` | shared, own lines | "Benchmarks" rewritten for W1 to W5; a new "Load generation" section; the lock count |
| `docs/reference/commands.md` | shared, own lines | The `just loadgen` row; `sync`, `lock`, `lock-check`, `fuzz`, `semver` and the snapshot rows say four lockfiles |
| `docs/reference/quality-gates.md` | shared, own lines | Gates 2, 4 and 6 name `loadgen/Cargo.toml`; the paragraph on the fuzz crate's own workspace covers both |
| `docs/reference/configuration.md` | shared, own lines | The root `Cargo.toml` row's exclude list; a `loadgen/Cargo.toml` row; the `.gitattributes` and version-pin rows name the lockfile |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | The lockfile rows and sentences name `loadgen/Cargo.lock`; the harness-dependency steps name the crate |
| `docs/operations/maintenance.md` | shared, own lines | The lock-upgrade and toolchain sentences count four; the release step says `loadgen/` needs no refresh after a library bump, since it names no workspace crate |
| `docs/how-to/develop-locally.md` | shared, own lines | The lockfile sentences |
| `docs/how-to/test-and-debug.md` | shared, own lines | The lock count in the snapshot section |
| `docs/tutorials/first-change.md` | shared, own lines | The lock count |
| `docs/project/repository-map.md` | shared, own lines | `loadgen/` in the tree and the responsibility table |
| `CHANGELOG.md` | shared, own lines | libpachinko Unreleased, Added: the W1 to W5 benches; pachinko Unreleased, Added: the load generator and lapse probe |
| `tickets/README.md` | shared, own lines | The index row's status |
| `tickets/T30-performance-evidence.md` | ticket | `status:` and hand-back notes |

`.gitattributes` is not touched: its `Cargo.lock` pattern has no slash, so it already
marks `loadgen/Cargo.lock` generated (T16's hand-back checked the same for `fuzz/`).
`tools.txt`, `deny.toml` and `.github/workflows/` are not touched: `loadgen/` is outside
the workspace and cargo-deny's graph, and no workflow runs it. T28 edits `Cargo.toml`,
`crates/libpachinko/Cargo.toml`, `docs/reference/testing.md`, `CHANGELOG.md` and
`tickets/README.md` on its own lane, so each is `shared, own lines` and the wave-2 rule
applies (`CONVENTIONS.md` §11): `origin/main` merged before the push, `Cargo.lock`
regenerated and never merged by hand, verification rerun and requoted.

## Steps

1. Create the worktree from `main` after T26 and T27 have merged (`README.md`, "How to
   pick up a ticket"), or, in a clean worktree already initialized on `main` whose branch
   carries nothing, create the branch there and skip `just initialize`. A new worktree
   runs `just initialize` (network, authorised by this step). Run `git fetch origin`
   (network, authorised by this step) so that `git show cb77f07:<path>` can read the
   prototype bench. Run `just check` on the untouched worktree and quote its last line as
   the baseline. Use the `plan-change` skill for this step and the next: read this
   ticket, T22's Context, decision 0019 as T22 wrote it, decision 0017,
   `tickets/T16-bench-and-fuzz.md` and `tickets/CONVENTIONS.md` §11 and §14.
2. **The benches.** Use the `rust-change` skill. Write
   `crates/libpachinko/benches/workloads.rs`, registered as `[[bench]] name = "workloads"`
   with `harness = false` beside `version`, which stays. One `Queue<MemoryRecords,
   FakeClock, CountingLeaseIds>` per sample, built outside the timed region by divan's
   `with_inputs`, with `Config::default()`, one publisher and one consumer joined, and
   P pockets pre-created, for P in `[10, 1_000, 100_000]` as the bench's `args`. Each
   workload reports throughput through divan's `ItemsCount` counter, so the table prints
   ops per second and nothing is computed by hand; every queue call's result passes
   through `divan::black_box`. The unit of one op, stated in the file's doc comment and
   in 0019: one library call. The workloads, each with 64-byte payloads and keys spread
   round-robin over the P pockets:
   - **W1 publish.** Each op publishes a fresh key into the next pocket.
   - **W2 supersede.** Keys pre-published once; each op publishes the same key at rank
     plus one.
   - **W3 pull 16, ack, release.** Pockets pre-filled with 16 keys; one cycle is a pull
     with limit 16, 16 acks and a release, 18 ops.
   - **W4 publish with every fifth op a consume step.** Four publishes, then one W3
     cycle; the counter counts the calls.
   - **W5 pull then lapse on tick.** One cycle is a pull that claims a pocket, the fake
     clock advanced past `lease_ttl`, and a `tick()` that lapses the lease; the clock
     step is inside the timed region, which the doc comment says, since the fake's store
     is one atomic write and nothing else can be measured without it. The lapsed pocket
     keeps the front, so the next cycle claims it again.

   Run `just bench` and quote the table for every workload and P. Then `just clippy`,
   which compiles the bench target in the gate, and `just doc`.
3. **The excluded crate.** Use the `rust-change` skill. Create `loadgen/` on the `fuzz/`
   pattern: `loadgen/Cargo.toml` names the package `pachinko-loadgen`, `version =
   "0.0.0"`, `publish = false`, `edition = "2024"`, `rust-version = "1.99"` (the pin's
   major.minor, as `fuzz/Cargo.toml` writes it), the `[lints.rust]` and `[lints.clippy]`
   tables copied verbatim from `fuzz/Cargo.toml` (an excluded crate inherits nothing;
   keep them aligned with the root table), and two dependencies with the decision-0007
   note each, in the form `fuzz/Cargo.toml` uses for libfuzzer-sys: `ureq`, no default
   features and no TLS, because the service is HTTP over loopback, and `serde_json`, to
   build the request bodies and read the replies as values without a derive. PR 39's
   pins at `cf1a6b2` were `ureq` 3.5.0 and `serde_json` 1.0.154; re-pin to the current
   releases on the day with `just lock` (network, authorised by this step), which writes
   `loadgen/Cargo.lock`; commit it. The root `Cargo.toml` excludes `loadgen`. The
   `Justfile` gains a `--manifest-path loadgen/Cargo.toml` line beside each `fuzz/` line
   in `sync`, `lock`, `lock-upgrade`, `lock-check`, `fix`, `fmt-check` and `clippy`: the
   three gate recipes are what the pattern requires, and the other four are what keeps
   the lockfile fetched, regenerated and formatted, since `lock-check` runs offline and
   fails on a cold cache that `sync` has not filled. The `fmt-check` hook's `files:`
   pattern in `.pre-commit-config.yaml` widens to `loadgen/`. `.gitignore` and
   `.dockerignore` take `loadgen/target/`. `renovate.json`'s `rust-version` regex manager
   lists `loadgen/Cargo.toml` and its description counts four places. Add the recipe,
   outside `check`'s list, with a comment in the `Justfile`'s voice:

   ```text
   # The load generator and lapse probe against a running `pachinko serve`, from
   # the excluded loadgen/ crate. Timings measure the machine: never a gate.
   loadgen *args:
       cargo run --manifest-path loadgen/Cargo.toml --release --locked -- "$@"
   ```

   `pyproject.toml` is not edited. Run `just lock-check`, `just fmt-check` and
   `just clippy` and quote each: the three gates now cover four lockfiles and three
   manifests.
4. **The load generator.** Use the `rust-change` skill. `loadgen` is one binary with
   five subcommands and these flags, each `--name value` with the default in brackets:
   `--base-url` (`http://127.0.0.1:8080`), `--seconds` (10), `--runs` (5), `--clients`
   (the workload's C), `--rows` (the batch's b), `--pockets` (1000), `--payload-bytes`
   (64). It joins its own publishers and consumers under ids that carry the process id,
   so two runs against one service never collide. Each client is a std thread with its
   own `ureq` agent on a keep-alive connection, in a closed loop: send, read the whole
   reply, parse it, count it, send the next. A reply counts only when its status is 2xx
   and its body parses and carries the outcome the workload expects (`created`,
   `new_key`, `superseded` or `stale` per row for a publish; a batch or `no_pocket` for a
   pull); anything else is an error, counted and printed, and a run with any error exits
   non-zero and prints no rate for it, so a service that refused connections or a route
   that answered 404 can never produce a number (`CONVENTIONS.md` §11, "Checks fail
   closed"). The report prints one line per run with its rate and error count and one
   median line; the hand-back quotes them. The workloads:
   - **`publish`**, C=64: single `PUT /pockets/{pocket_key}/records/{record_key}` with
     fresh keys over the P pockets; records per second.
   - **`batch`**, b=256 at C=8 and b=1,024 at C=64: `POST /publishers/{publisher_id}/publish`
     with b rows of fresh keys over the P pockets; rows per second, counting only rows
     whose outcome the reply carries.
   - **`consume`**, C=64: pockets pre-filled by the same generator (the fill is not
     timed); each consumer pulls with limit `max_pull_batch`, acks each record of the
     batch back to back on its connection, releases, and pulls again; records acked per
     second, and a `no_pocket` reply ends that consumer's loop.
   - **`mix`**: half the clients publish single records, half consume, concurrently;
     both rates printed.
   - **`lapse`**, the probe: the service started with a short lease and sweep
     (`--lease-ttl-ms 2000 --sweep-ms 1000`, recorded in 0019 beside the figure). Before
     any load pocket exists the probe publishes one record into a probe pocket, so that
     pocket holds the smallest position and keeps the front on a lapse
     (`LeaseExpiryKeepsPocketAtFront`). Consumer A pulls it, reads `expires_at` from
     `GET /consumers/{consumer_id}/lease`, and never renews. Batch publishers at b=256,
     C=8 then load the other pockets. Consumer B polls `pull` every millisecond; a load
     pocket it claims it releases at once (the release sends that pocket to the back,
     never ahead of the probe pocket). The lag is B's claim instant minus `expires_at`,
     both read on this machine; the service's clock is `SystemTime` behind the monotonic
     clamp, so the two are comparable to a millisecond. Repeated `--runs` times; p50 and
     max printed. The probe's expected floor is the sweep interval, and 0019 says so.

   Start the service in a second shell with `cargo run --release --locked -p pachinko --
   serve` and the flags the run needs, or the recipe T24 added if one exists, and record
   the flags beside each figure. Run every workload in the Verification block, five runs
   each, and quote the lines. Run the negative control first: `just loadgen publish` at
   a closed port exits non-zero and prints no rate.
5. **The record and the handbook.** Use the `review-docs` skill. Add an "Evidence from
   the MVP" section to `docs/decisions/0019-service-runtime-tokio-and-http.md`, after its
   evidence section, that names the machine and harness in the sentence form of this
   ticket's Context (the agent's own machine, toolchain and date), the commit measured,
   the service flags, the unit of each figure, and then: a table of W1 to W5 at each P,
   each beside its prototype figure; a table of the loopback workloads, each beside its
   prototype figure where one exists (consume and mix have none, and the row says so);
   the lapse lag p50 and max beside the prototype lags; the quotient of the MVP's
   b=256, C=8 rate over 1,565,592 rows/s, stated as the number the maintainer's ratio is
   read against; and the caveats from Context. Rewrite "Benchmarks" in
   `docs/reference/testing.md` for the five workloads and add "Load generation" (what
   `just loadgen` measures, how a run fails closed, the lock it checks), the
   `just loadgen` row in `docs/reference/commands.md`, the `loadgen/` paragraph in
   decision 0007, the rows in `docs/project/repository-map.md` and
   `docs/reference/configuration.md`, and the lockfile count, three to four, on every
   page Files touched names; `rg -n 'three lock' docs/` and `rg -n 'all three' docs/`
   find them, and only the lockfile sentences change. `CHANGELOG.md`: both crates'
   Unreleased, Added. Run `just check-docs`.
6. **Adversarial review** (`CONVENTIONS.md` §14): the whole diff and the quoted outputs
   against this ticket's Goal, Files touched and Acceptance criteria, T22's evidence
   table and the new section of 0019. Ask: whether any number in 0019 is one the harness
   did not print under a quoted command and commit; whether a bench measures the fake
   clock, the setup or the counter rather than the queue (the input built outside the
   timed region, `black_box` on every result, the W5 clock step declared); whether the
   load generator can count a reply it never read or parsed, or print a rate for a run
   with an error, or exit zero against a closed port; whether the lapse probe's
   `expires_at` and claim instant are read from clocks that can be compared; whether
   `just loadgen` or a bench reached `check`'s recipe list, a CI job or a hook; whether
   `loadgen/Cargo.toml`'s lint tables match the root's and `fuzz/`'s; whether the three
   gate recipes and the four lock recipes each name `loadgen/Cargo.toml`; and whether a
   page outside Files touched still counts three lockfiles. Codex `gpt-6-astra`
   reviewers run read-only with fresh contexts, one lens each; separate triage subagents
   write verdicts; valid fixes are applied one writer at a time; two rounds at most.
   Then `just check`, and commit on the ticket branch with the hand-back notes quoting
   each output headed by the commit it ran at. Before the push: `git fetch origin`
   (network, authorised by this step), merge `origin/main` if T28 landed first,
   regenerate `Cargo.lock` with `just lock` (network, authorised by this step) if the
   merge touched it, rerun the Verification block and requote it. **Authorisation
   required: push and open a pull request.** Stop and ask.

## Acceptance criteria

- `just bench` prints a throughput line for each of W1 to W5 at P = 10, 1,000 and
  100,000, quoted, with the unit of one op stated in the bench's doc comment.
- `just lock-check`, `just fmt-check` and `just clippy` each cover `loadgen/Cargo.toml`
  and are quoted green; `loadgen/Cargo.lock` is committed; `just sync`, `just lock`,
  `just lock-upgrade` and `just fix` name the manifest too.
- `just loadgen` runs from the `Justfile` outside `check`'s recipe list;
  `pyproject.toml` is unchanged; no workflow runs a bench or the load generator.
- `just loadgen publish` against a closed port exits non-zero and prints no rate,
  quoted; every measured run prints one line per run and a median, with zero errors.
- The publish (C=64), batch (b=256 at C=8; b=1,024 at C=64), consume (C=64), mix and
  lapse workloads are each quoted from five runs, headed by the commit and the service
  flags.
- Decision 0019's "Evidence from the MVP" section carries every quoted number beside its
  prototype figure, the machine and harness sentence, the unit of each figure, the
  quotient against 1,565,592 rows/s, and the caveats; no number appears there that the
  hand-back does not quote.
- Every page Files touched names counts four lockfiles, and `rg -n 'three lock' docs/`
  finds no lockfile sentence.
- `just check-docs` and `just check` are green.
- The adversarial review in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

The service runs in a second shell for the loopback workloads, started with
`cargo run --release --locked -p pachinko -- serve` and the flags quoted beside each
figure; the `lapse` runs use `--lease-ttl-ms 2000 --sweep-ms 1000`.

```sh
just lock-check
just fmt-check
just clippy
just bench
just loadgen --help
just loadgen publish --base-url http://127.0.0.1:1 --seconds 1 --runs 1
just loadgen publish --clients 64 --seconds 10 --runs 5
just loadgen batch --rows 256 --clients 8 --seconds 10 --runs 5
just loadgen batch --rows 1024 --clients 64 --seconds 10 --runs 5
just loadgen consume --clients 64 --seconds 10 --runs 5
just loadgen mix --clients 64 --seconds 10 --runs 5
just loadgen lapse --runs 5
rg -n 'three lock' docs/
just check-docs
just check
git status --short
```

The run against port 1 is the negative control and must exit non-zero; the `rg` prints
nothing, and the positive control is the same command on the untouched worktree, which
prints the sentences this ticket rewrites (`CONVENTIONS.md` §11, "Checks fail closed").

## Hand-back notes

Not started. The executing agent writes this section: each quoted output headed by the
commit it ran at, with its exit status (`CONVENTIONS.md` §11), and for each adversarial
checkpoint the harness and model, the findings, each verdict, what was applied and what
was handed back (`CONVENTIONS.md` §14).

## Open points

- The Flight reopen ratio: 0019 names the 1,565,592 rows/s figure and this ticket reports
  the MVP's rate and the quotient; whether the quotient reopens the wire is the
  maintainer's call, and the ratio is not recorded anywhere until the maintainer sets it.
- The in-process unit of an op is this ticket's definition, one library call; the
  prototypes' definition is not in the repository, so the W1 to W5 comparison is read as
  an order of magnitude, not a ratio. If the maintainer's reference generator or bench
  defines the unit differently, 0019 says which.
- Nothing is measured on Linux; the container image (`just image`) is the place a Linux
  figure would come from, and it is not this ticket's.
- RSS under sustained batch publishing is not bounded (`NeverRefusedForCapacity`, open
  question 3); if the executing agent reads it with `ps -o rss= -p <pid>` after a 10 s
  batch run, 0019 reports it beside the 8,191 MiB figure, as an observation.
- Closed-loop clients understate tail latency; an open-loop generator and a latency
  histogram are a later ticket if the maintainer wants them.
- The `version` smoke bench from T16 stays; removing it once W1 to W5 exist is the
  maintainer's choice.
- `loadgen/` is a fourth lockfile for Renovate's lock-file maintenance and for the
  release runbook; this ticket updates both pages, and `loadgen/` needs no refresh after
  a library bump because it names no workspace crate.
