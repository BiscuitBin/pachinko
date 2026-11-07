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

   **Adversarial review** (`CONVENTIONS.md` §14): the load generator, uncommitted,
   against this ticket's Goal, Files touched and Acceptance criteria, the workloads above
   and the routes T24 and T27 serve, before any figure is measured: every number 0019
   gains rests on it. Ask: whether a reply can count without a 2xx status and a parsed
   body carrying the expected outcome; whether a run with any error can print a rate or
   exit zero; whether a batch counts rows the reply did not carry; whether the timed
   region includes the joins, the fill or other setup; whether ids can collide across
   runs or processes; whether the lapse probe's pocket can lose the front, or B's claim
   instant and `expires_at` come from clocks that cannot be compared; whether every
   client is closed-loop on its own keep-alive connection as stated; and whether
   `loadgen/Cargo.toml`'s lint tables match the root's and `fuzz/`'s. Two reviewer
   subagents with fresh contexts run in parallel, one lens each, `conformance` and
   `breakage`, each running `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T30-1/findings-<lens>.md "<brief>" < /dev/null` (§14's command; `-o`
   writes only the reviewer's last message, and a read-only reviewer writes no file of
   its own); separate triage subagents check every finding against the tree and write
   `ai_tmp/review/T30-1/verdicts.md`, each finding valid with its fix or invalid with the
   reason; one writer applies the valid fixes one at a time, only within Files touched,
   and hands back any fix outside it; if a high-severity finding was valid, a second
   round reviews the fix diff and writes beside the first with a `-2` suffix; two rounds
   at most.

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
   page outside Files touched still counts three lockfiles. Two reviewer subagents with
   fresh contexts run in parallel, one lens each, `conformance` and `breakage`, each
   running `codex exec -m gpt-6-astra -s read-only -C <worktree> -o
   ai_tmp/review/T30-2/findings-<lens>.md "<brief>" < /dev/null` (§14's command; `-o`
   writes only the reviewer's last message, and a read-only reviewer writes no file of
   its own); separate triage subagents check every finding against the tree and write
   `ai_tmp/review/T30-2/verdicts.md`, each finding valid with its fix or invalid with the
   reason; one writer applies the valid fixes one at a time, only within Files touched,
   and hands back any fix outside it; if a high-severity finding was valid, a second
   round reviews the fix diff and writes beside the first with a `-2` suffix; two rounds
   at most. Then `just check`, and commit on the ticket branch with the hand-back notes
   quoting each output headed by the commit it ran at. Before the push: `git fetch
   origin` (network, authorised by this step), merge `origin/main` if T28 landed first,
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
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
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

### Baseline

At `1fdaab3` (T22 to T27 and T29 landed), after `just initialize` (exit 0; "Secondary
worktree: skipping install-hooks."), `just check` on the untouched worktree (exit 0):

```text
All checks passed and the worktree is unchanged.
```

### Wave overrides

- Worked on branch `wave/t30` in the lead's lane worktree from `1fdaab3`, not on
  `ticket/t30-performance-evidence` from `main`. `just initialize` ran there, as the lead
  asked. The prototype head `cb77f07` already resolved, so `git fetch origin` was not run.
- `git fetch origin`, merging `origin/main`, push and pull request: not run: one pull
  request at T31. The lead merges this lane into `mvp-inimal-but-sufficient`, and the
  Verification block is rerun at that merge commit when asked.
- The §14 reviews, triage and fix writer were run by the wave lead; this agent stopped
  uncommitted at each checkpoint, read the fix diff and reran its checks.

### Deviations from Files touched and the steps

- The `Justfile`'s `format` recipe also gains the `loadgen/` line beside its `fuzz/`
  line, which the ticket's list of seven recipes omits (approved by the lead).
- `Justfile`: a `test-loadgen` recipe, `cargo test --manifest-path loadgen/Cargo.toml
  --locked`, outside `check`. `just test` cannot reach the excluded crate's unit tests,
  and AGENTS.md forbids an ad-hoc command (asked for by the lead after T30-1; see Open
  points).
- `loadgen` has one flag beyond step 4's list, `--fill` (default 1,000,000), the size of
  `consume`'s untimed fill.
- Port 8080 is held by another process on this machine, so every service ran with
  `--listen 127.0.0.1:8090` and every loadgen run with `--base-url http://127.0.0.1:8090`.
- `renovate.json`'s toolchain description says "four places", as the ticket says, though
  its sentence lists five files in three clauses.

### Interpretations (step 2 and step 4)

- The benches run a fixed number of calls per sample over one queue (`sample_count = 20`,
  `sample_size = 1`), so the P = 100,000 setup stays outside the timed region and the
  run takes about 35 s. A fully acked pocket idles on its release, so W3 and W4 run
  `min(1,000, P)` cycles; the third consecutive lapse parks a pocket under
  `Config::default()`, so W5 runs `min(1,000, 3 P)`. The bench's doc comment says both.
- `consume`: a `no_pocket` inside the window ends that consumer's loop, as step 4 says.
  The default `--fill` outlasted every recorded window. `mix`: even threads publish,
  odd threads consume with limit 100, and a mixed consumer's `no_pocket` sleeps 1 ms
  and pulls again.
- `lapse` runs five times on one service by draining the queue after each claim,
  untimed, and fails a run whose probe pocket is not the front. Every rule that resets a
  pocket's consecutive expiries also moves it, so the drain is what keeps the next probe
  pocket at the front.
- Step 4 calls the sweep interval the probe's expected floor; the ticker lapses at the
  first tick after `expires_at`, so the sweep interval is the expected ceiling of the
  unloaded lag, and 0019 says that.

### Step 2: the benches

The quoted run is the re-measurement after T30-2 (see "The re-measurement" below), at
`1fdaab3` plus the uncommitted change, from `ai_tmp/wave/T30/measure.sh bench`:

```text
quiet: load 2.32 2.91 4.64
== commit 1fdaab3 plus the uncommitted change; 2026-11-07T15:51:03Z
before: load 2.32 2.91 4.64
$ just bench
cargo bench --workspace --all-features --locked --bench '*' "$@"
    Finished `bench` profile [optimized] target(s) in 0.02s
     Running benches/version.rs (target/release/deps/version-3ff7ca136ee03e5e)
Timer precision: 41 ns
version     fastest       │ slowest       │ median        │ mean          │ samples │ iters
╰─ version  0.22 ns       │ 0.589 ns      │ 0.222 ns      │ 0.25 ns       │ 100     │ 1638400

     Running benches/workloads.rs (target/release/deps/workloads-3dddeb769b405c4b)
Timer precision: 41 ns
workloads                  fastest       │ slowest       │ median        │ mean          │ samples │ iters
├─ w1_publish                            │               │               │               │         │
│  ├─ 10                   1.803 ms      │ 2.196 ms      │ 1.915 ms      │ 1.924 ms      │ 20      │ 20
│  │                       5.544 Mitem/s │ 4.551 Mitem/s │ 5.22 Mitem/s  │ 5.196 Mitem/s │         │
│  ├─ 1000                 800.3 µs      │ 915.1 µs      │ 862.9 µs      │ 857.1 µs      │ 20      │ 20
│  │                       12.49 Mitem/s │ 10.92 Mitem/s │ 11.58 Mitem/s │ 11.66 Mitem/s │         │
│  ╰─ 100000               2.826 ms      │ 3.034 ms      │ 2.938 ms      │ 2.935 ms      │ 20      │ 20
│                          3.538 Mitem/s │ 3.295 Mitem/s │ 3.402 Mitem/s │ 3.406 Mitem/s │         │
├─ w2_supersede                          │               │               │               │         │
│  ├─ 10                   835.9 µs      │ 1.06 ms       │ 866 µs        │ 885.7 µs      │ 20      │ 20
│  │                       11.96 Mitem/s │ 9.432 Mitem/s │ 11.54 Mitem/s │ 11.29 Mitem/s │         │
│  ├─ 1000                 1.104 ms      │ 1.213 ms      │ 1.147 ms      │ 1.154 ms      │ 20      │ 20
│  │                       9.051 Mitem/s │ 8.238 Mitem/s │ 8.712 Mitem/s │ 8.659 Mitem/s │         │
│  ╰─ 100000               2.623 ms      │ 3.147 ms      │ 2.707 ms      │ 2.754 ms      │ 20      │ 20
│                          3.812 Mitem/s │ 3.176 Mitem/s │ 3.693 Mitem/s │ 3.63 Mitem/s  │         │
├─ w3_pull_ack_release                   │               │               │               │         │
│  ├─ 10                   52.58 µs      │ 86.79 µs      │ 53.24 µs      │ 55.19 µs      │ 20      │ 20
│  │                       3.423 Mitem/s │ 2.073 Mitem/s │ 3.38 Mitem/s  │ 3.261 Mitem/s │         │
│  ├─ 1000                 5.459 ms      │ 6.09 ms       │ 5.665 ms      │ 5.665 ms      │ 20      │ 20
│  │                       3.297 Mitem/s │ 2.955 Mitem/s │ 3.177 Mitem/s │ 3.177 Mitem/s │         │
│  ╰─ 100000               6.375 ms      │ 7.311 ms      │ 6.672 ms      │ 6.787 ms      │ 20      │ 20
│                          2.823 Mitem/s │ 2.462 Mitem/s │ 2.697 Mitem/s │ 2.651 Mitem/s │         │
├─ w4_publish_and_consume                │               │               │               │         │
│  ├─ 10                   47.41 µs      │ 63.7 µs       │ 50.72 µs      │ 52.05 µs      │ 20      │ 20
│  │                       4.639 Mitem/s │ 3.453 Mitem/s │ 4.336 Mitem/s │ 4.226 Mitem/s │         │
│  ├─ 1000                 5.59 ms       │ 6.341 ms      │ 5.899 ms      │ 5.921 ms      │ 20      │ 20
│  │                       3.935 Mitem/s │ 3.469 Mitem/s │ 3.729 Mitem/s │ 3.714 Mitem/s │         │
│  ╰─ 100000               8.149 ms      │ 9.552 ms      │ 8.564 ms      │ 8.652 ms      │ 20      │ 20
│                          2.699 Mitem/s │ 2.303 Mitem/s │ 2.568 Mitem/s │ 2.542 Mitem/s │         │
╰─ w5_pull_then_lapse                    │               │               │               │         │
   ├─ 10                   6.707 µs      │ 138.4 µs      │ 8.04 µs       │ 14.48 µs      │ 20      │ 20
   │                       8.944 Mitem/s │ 433.2 Kitem/s │ 7.461 Mitem/s │ 4.141 Mitem/s │         │
   ├─ 1000                 300.3 µs      │ 351.4 µs      │ 330.4 µs      │ 332.1 µs      │ 20      │ 20
   │                       6.659 Mitem/s │ 5.69 Mitem/s  │ 6.052 Mitem/s │ 6.021 Mitem/s │         │
   ╰─ 100000               485.1 µs      │ 557.4 µs      │ 525.2 µs      │ 522.8 µs      │ 20      │ 20
                           4.122 Mitem/s │ 3.587 Mitem/s │ 3.807 Mitem/s │ 3.825 Mitem/s │         │

bench exit 0
after: load 2.94 3.02 4.63
```

`version` printed its one line as before.

### Step 3: the gates over the fourth lockfile

`just lock` re-pinned `loadgen/Cargo.lock` at ureq 3.5.1 and serde_json 1.0.154, the
current releases; `Cargo.lock` and `pixi.lock` did not move. At `1fdaab3` plus the
uncommitted change, after T30-1's fixes, each exit 0:

```text
$ just lock-check
cargo update --workspace --locked --offline
     Locking 0 packages to highest Rust 1.99 compatible versions
cargo update --workspace --locked --offline --manifest-path fuzz/Cargo.toml
     Locking 0 packages to highest Rust 1.99 compatible versions
note: pass `--verbose` to see 1 unchanged dependencies behind latest
cargo update --workspace --locked --offline --manifest-path loadgen/Cargo.toml
     Locking 0 packages to highest Rust 1.99 compatible versions
pixi lock --check --offline --dry-run
✔ Lock-file was already up-to-date
$ just fmt-check
cargo fmt --all --check
cargo fmt --manifest-path fuzz/Cargo.toml --check
cargo fmt --manifest-path loadgen/Cargo.toml --check
$ just clippy
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
cargo clippy --manifest-path loadgen/Cargo.toml --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
```

### T30-1: the load generator (step 4)

Harness and model: two reviewer lenses, `conformance` and `breakage`, each
`codex exec -m gpt-6-astra -s read-only` (exit 0), with separate triage and one writer,
run by the wave lead; outputs under `ai_tmp/review/T30-1/`. 7 findings, 7 valid, none
high, so no second round.

| Finding | Verdict | One-line reason |
| --- | --- | --- |
| C1 | valid, medium | `mix` defaulted to a pull limit of 1, not `max_pull_batch` (100), so its figure measured another workload |
| C2 | valid, low | Nothing checked that the batch load ran before `expires_at`, so an unloaded lapse run could count |
| C3 | valid, low | The fill and the drain kept only their first error, against "counted and printed" |
| B1 | valid, low | Same root as C2, from the breakage side (re-rated from high) |
| B2 | valid, low | The clock started after the barrier released the clients, so work before `start` could count |
| B3 | valid, low | A claim returned past `CLAIM_BOUND` was accepted, against the bound's own contract |
| B4 | valid, low | Ids carried only the process id, which macOS reuses |

Applied by the writer: `mix` defaults to (64, 100) (C1); a lapse run fails unless a
batch was counted before `expires_at` (C2, B1); the fill and the drain keep every error
(C3); ready and go barriers start the clock before any client steps (B2); a claim past
`CLAIM_BOUND` releases B's lease and is an error (B3); ids carry a nonce of the process
id and the system clock's nanoseconds (B4). Handed back: nothing. The lead then asked
for the `test-loadgen` recipe, since no recipe ran loadgen's unit tests.

Rerun after the fixes, at `1fdaab3` plus the uncommitted change: the three gates above,
`just loadgen --help` (exit 0, the usage text), and:

```text
$ just test-loadgen
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
exit 0
$ just loadgen publish --base-url http://127.0.0.1:1 --seconds 1 --runs 1
cargo run --manifest-path loadgen/Cargo.toml --release --locked -- "$@"
    Finished `release` profile [optimized] target(s) in 0.01s
     Running `loadgen/target/release/pachinko-loadgen publish --base-url 'http://127.0.0.1:1' --seconds 1 --runs 1`
publish against http://127.0.0.1:1: C=64 b=1 P=1000 payload 64 bytes, 1 runs, 1s each
run 1: 64 errors, no rate
error: recipe `loadgen` failed on line 116 with exit code 1
(plus 64 lines "  error: POST /publishers/lg…-r1-tN: io: Connection refused (os error 61)")
exit 1
```

### Step 4: the loopback workloads

These are the re-measured figures (see "The re-measurement" below). Each workload ran
against a fresh release service, `./target/release/pachinko serve --listen
127.0.0.1:8090` plus the flags each block names (the binary
`cargo build --release --locked -p pachinko` builds). `ai_tmp/wave/T30/measure.sh` ran
each block. Before the block it waited, for up to 600 s, until `uptime`'s 1-minute load
was at or below 4 (`quiet:`), and it recorded the load `before:` and `after:`. It started
and stopped the service through `serve.sh`, which reads `ps -o rss=` before the stop. The
script exits non-zero if the service does not start, the stop finds no service, or
loadgen exits non-zero (T30-2's B2), and every block exited 0. The service's source is
`1fdaab3`'s; T30 changes no service file. No other lane and no `just check` ran during
the measurements. After 9 GiB the batch service took longer than the 10 s stop poll to
exit, so the script sent SIGKILL (`killed9`) after the figures had been read.

```text
quiet: load 2.70 2.96 4.60
== commit 1fdaab3 plus the uncommitted change; 2026-11-07T15:51:33Z
before: load 2.70 2.96 4.60
pid 98121: ./target/release/pachinko serve --listen 127.0.0.1:8090 
listening on 127.0.0.1:8090
$ just loadgen publish --clients 64 --seconds 10 --runs 5 --base-url http://127.0.0.1:8090
publish against http://127.0.0.1:8090: C=64 b=1 P=1000 payload 64 bytes, 5 runs, 10s each
run 1: 61952 records/s (619667 records) over 10.002 s, 0 errors
run 2: 60439 records/s (604578 records) over 10.003 s, 0 errors
run 3: 60468 records/s (604755 records) over 10.001 s, 0 errors
run 4: 58810 records/s (588220 records) over 10.002 s, 0 errors
run 5: 58657 records/s (586691 records) over 10.002 s, 0 errors
median of 5 runs: 60439 records/s
loadgen exit 0
after: load 8.43 4.47 5.06
rss_kib 1083488
stopped
```

```text
quiet: load 3.99 3.99 4.83
== commit 1fdaab3 plus the uncommitted change; 2026-11-07T15:53:39Z
before: load 3.99 3.99 4.83
pid 99433: ./target/release/pachinko serve --listen 127.0.0.1:8090 
listening on 127.0.0.1:8090
$ just loadgen batch --rows 256 --clients 8 --seconds 10 --runs 5 --base-url http://127.0.0.1:8090
batch against http://127.0.0.1:8090: C=8 b=256 P=1000 payload 64 bytes, 5 runs, 10s each
run 1: 1193283 rows/s (11935744 rows) over 10.002 s, 0 errors
run 2: 966035 rows/s (9663232 rows) over 10.003 s, 0 errors
run 3: 904824 rows/s (9051904 rows) over 10.004 s, 0 errors
run 4: 864404 rows/s (8646912 rows) over 10.003 s, 0 errors
run 5: 797480 rows/s (7976960 rows) over 10.003 s, 0 errors
median of 5 runs: 904824 rows/s
loadgen exit 0
after: load 4.37 4.11 4.82
rss_kib 9549952
killed9
stopped
```

```text
quiet: load 3.72 4.02 4.75
== commit 1fdaab3 plus the uncommitted change; 2026-11-07T15:55:18Z
before: load 3.72 4.02 4.75
pid 527: ./target/release/pachinko serve --listen 127.0.0.1:8090 --max-batch-rows 1024
listening on 127.0.0.1:8090
$ just loadgen batch --rows 1024 --clients 64 --seconds 10 --runs 5 --base-url http://127.0.0.1:8090
batch against http://127.0.0.1:8090: C=64 b=1024 P=1000 payload 64 bytes, 5 runs, 10s each
run 1: 1062254 rows/s (10692608 rows) over 10.066 s, 0 errors
run 2: 870269 rows/s (8768512 rows) over 10.076 s, 0 errors
run 3: 906101 rows/s (9135104 rows) over 10.082 s, 0 errors
run 4: 847834 rows/s (8545280 rows) over 10.079 s, 0 errors
run 5: 811463 rows/s (8190976 rows) over 10.094 s, 0 errors
median of 5 runs: 870269 rows/s
loadgen exit 0
after: load 5.52 4.51 4.89
rss_kib 14631216
stopped
```

```text
quiet: load 3.94 4.21 4.75
== commit 1fdaab3 plus the uncommitted change; 2026-11-07T15:57:22Z
before: load 3.94 4.21 4.75
pid 2160: ./target/release/pachinko serve --listen 127.0.0.1:8090 
listening on 127.0.0.1:8090
$ just loadgen consume --clients 64 --seconds 10 --runs 5 --base-url http://127.0.0.1:8090
consume against http://127.0.0.1:8090: C=64 b=100 P=1000 payload 64 bytes, 5 runs, 10s each after a fill of 1000000
run 1: 55925 acked/s (563200 acked) over 10.071 s, 0 errors
run 2: 58183 acked/s (582400 acked) over 10.010 s, 0 errors
run 3: 58094 acked/s (582400 acked) over 10.025 s, 0 errors
run 4: 59147 acked/s (595200 acked) over 10.063 s, 0 errors
run 5: 56891 acked/s (569600 acked) over 10.012 s, 0 errors
median of 5 runs: 58094 acked/s
loadgen exit 0
after: load 7.73 5.24 5.10
rss_kib 1042880
stopped
```

```text
quiet: load 3.73 4.57 4.85
== commit 1fdaab3 plus the uncommitted change; 2026-11-07T15:59:32Z
before: load 3.73 4.57 4.85
pid 3775: ./target/release/pachinko serve --listen 127.0.0.1:8090 
listening on 127.0.0.1:8090
$ just loadgen mix --clients 64 --seconds 10 --runs 5 --base-url http://127.0.0.1:8090
mix against http://127.0.0.1:8090: C=64 b=100 P=1000 payload 64 bytes, 5 runs, 10s each
run 1: 30175 published/s (302302 published), 27849 acked/s (278992 acked) over 10.018 s, 0 errors
run 2: 30366 published/s (303754 published), 29320 acked/s (293298 acked) over 10.003 s, 0 errors
run 3: 30701 published/s (307381 published), 29952 acked/s (299879 acked) over 10.012 s, 0 errors
run 4: 29703 published/s (297281 published), 29084 acked/s (291082 acked) over 10.008 s, 0 errors
run 5: 30238 published/s (303773 published), 29871 acked/s (300092 acked) over 10.046 s, 0 errors
median of 5 runs: 30238 published/s, 29320 acked/s
loadgen exit 0
after: load 8.19 5.63 5.22
rss_kib  32224
stopped
```

```text
quiet: load 3.94 4.91 4.99
== commit 1fdaab3 plus the uncommitted change; 2026-11-07T16:01:45Z
before: load 3.94 4.91 4.99
pid 5207: ./target/release/pachinko serve --listen 127.0.0.1:8090 --lease-ttl-ms 2000 --sweep-ms 1000
listening on 127.0.0.1:8090
$ just loadgen lapse --runs 5 --base-url http://127.0.0.1:8090
lapse against http://127.0.0.1:8090: C=8 b=256 P=1000 payload 64 bytes, 5 runs, each until B's claim
run 1: lag 16 ms (expires_at 1794067308667, claimed at 1794067308683), 3081984 load rows, 0 errors
run 2: lag 310 ms (expires_at 1794067364428, claimed at 1794067364738), 3335168 load rows, 0 errors
run 3: lag 68 ms (expires_at 1794067424728, claimed at 1794067424796), 3002368 load rows, 0 errors
run 4: lag 843 ms (expires_at 1794067479007, claimed at 1794067479850), 3983872 load rows, 0 errors
run 5: lag 288 ms (expires_at 1794067551633, claimed at 1794067551921), 3151104 load rows, 0 errors
lapse lag over 5 runs: p50 288 ms, max 843 ms
loadgen exit 0
after: load 12.89 9.45 7.03
rss_kib 1768304
stopped
```

The RSS observation, one 10 s run on a fresh service:

```text
quiet: load 3.53 4.15 4.74
== commit 1fdaab3 plus the uncommitted change; 2026-11-07T15:57:05Z
before: load 3.53 4.15 4.74
pid 1946: ./target/release/pachinko serve --listen 127.0.0.1:8090 
listening on 127.0.0.1:8090
$ just loadgen batch --rows 256 --clients 8 --seconds 10 --runs 1 --base-url http://127.0.0.1:8090
batch against http://127.0.0.1:8090: C=8 b=256 P=1000 payload 64 bytes, 1 runs, 10s each
run 1: 1173080 rows/s (11734528 rows) over 10.003 s, 0 errors
median of 1 runs: 1173080 rows/s
loadgen exit 0
after: load 3.76 4.18 4.74
rss_kib 4617840
stopped
```

### Step 5: the record and the handbook

`docs/decisions/0019-service-runtime-tokio-and-http.md` gains "Evidence from the MVP"
with the figures above, from the `just bench` run quoted in step 2 and the loopback runs
in step 4. The quotient is 904,824 / 1,565,592 = 0.58.

At `1fdaab3` plus the uncommitted change, `just check-docs` (exit 0):

```text
Validated 46 pages and 47 canonical topics.
```

`rg -n 'three lock' docs/` (exit 0, one line, outside Files touched):

```text
docs/decisions/0016-release-process.md:63:The recipe checks all three locks first and runs offline because cargo-semver-checks
```

The positive control, `git grep -n 'three lock' 1fdaab3 -- docs/`, printed twelve lines on
the untouched tree, eleven of them on pages this ticket rewrites.

Handed back, outside Files touched: `docs/decisions/0016-release-process.md:63` ("all
three locks") and `docs/decisions/0018-renovate.md:49` (the toolchain "in three places",
which `loadgen/Cargo.toml`'s `rust-version` joins) still count three.

### T30-2: the whole diff (step 6)

Harness and model: two reviewer lenses, `conformance` and `breakage`, each
`codex exec -m gpt-6-astra -s read-only` (exit 0), with separate triage and one writer,
run by the wave lead; outputs under `ai_tmp/review/T30-2/`. 6 findings, 6 valid, none
high.

| Finding | Verdict | One-line reason |
| --- | --- | --- |
| C1 | valid, low | The step 2 quote left out W5's P=1,000 and P=100,000 rows |
| C2 | valid, low | `consume` failed the run on `no_pocket`, where step 4 says it ends that consumer's loop |
| C3 | valid, low, outside Files touched | `docs/decisions/0016-release-process.md:63` still says "all three locks", so `rg -n 'three lock' docs/` is not empty |
| B1 | valid, low | The client followed redirects, so a 3xx could be counted as the redirect target's reply |
| B2 | valid, low, outside Files touched | The scratch `measure.sh` ignored a failed service start and loadgen's exit status |
| B3 | valid, low | Same as C1 |

Applied by the writer: the step 2 quote gives every W5 row (C1, B3); `consume` ends that
consumer's loop on `no_pocket` (`WhenEmpty::Finish`), and `testing.md` says so (C2);
`Client::new` sets `max_redirects(0)` (B1). Applied by this agent, in scratch: B2's fix
to `measure.sh` (it is not committed). Its failure paths were checked. With port 8090
already bound, the run printed `serve start failed` and exited 1. With an unknown
workload, it printed `loadgen exit 2` and exited 2.

Handed back: C3, `docs/decisions/0016-release-process.md:63`, to T31. Until T31 lands,
the acceptance criterion "`rg -n 'three lock' docs/` finds no lockfile sentence" is
unmet, and the one line it prints is this one.

Round 2 reviewed the re-measured evidence and the fix diff. Harness and model: the same
two lenses, each `codex exec -m gpt-6-astra -s read-only` (exit 0), with triage and the
writer run by the wave lead; outputs `ai_tmp/review/T30-2/*-2.md` and
`verdicts-2.md`. 4 findings, 4 valid, all low. Two rounds is the cap.

| Finding | Verdict | One-line reason |
| --- | --- | --- |
| C1-2 | valid, low | The re-quoted step 2 table still stopped after W5's P=10 row, though the record said every W5 row was quoted |
| C2-2 | valid, low | 0019 said no figure was taken under outside load, which a total load average cannot show |
| B1-2 | valid, low | Same as C1-2 |
| B2-2 | valid, low | Same as C2-2 |

Applied by the writer: the W5 P=1,000 and P=100,000 rows quoted from `m-bench.log`
(C1-2, B1-2); 0019's caveat says only the total 1-minute load was checked at or below 4,
so other background load is not ruled out (C2-2, B2-2). After round 2 this agent also
quoted the nine lines of the bench's build and harness preamble that the step 2 quote had
cut without a mark. Handed back: nothing new.

At the lane commit the pre-commit hook's EditorConfig check failed on
`loadgen/src/main.rs`: the usage text was indented by two spaces, not a multiple of
four. `just check`'s `lint` had not seen it, because `prek run --all-files` reads only
files Git tracks, and `loadgen/` was untracked until it was staged. The usage text is
now indented by four, and the hook passes.

### The re-measurement

The first measurements started with the 1-minute load between 4.62 and 11.88, mostly the
decay of the previous workload run back to back. At the lead's direction everything was
measured again after T30-2's fixes, with the wait for load at or below 4 and the load
recorded before and after: `just bench` and the six workloads above. Decision 0019
quotes only the re-measured figures, and the first run's logs stay in
`ai_tmp/wave/T30/old/`. Every re-measured workload started at a 1-minute load between
2.32 and 3.99.

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
- `just test-loadgen` runs the excluded crate's unit tests by hand only. Whether a gate
  (`just check`'s list in `pyproject.toml`, or a CI job) should run them is the
  maintainer's call; the ticket keeps `loadgen/` out of `check`, and only its lint,
  format and lock gates run there.
