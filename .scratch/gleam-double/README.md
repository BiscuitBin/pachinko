# Tickets for a Gleam double of libpachinko

These seven tickets build a second double of the domain `docs/specs/pachinko.allium`
defines, in Gleam: a pure core that holds every rule, behind one owner actor that
serializes every action and every tick. It covers the pocket queue, leases, pull, ack,
nack, renew, release, the lease's end, expiry, parking and decay. The HTTP service is out
of scope, and so is a real ticker: tests set a fake clock and send `Tick` themselves.

The tickets follow the format and the conventions of `tickets/`, as the Python double's
did (`.scratch/python-double/README.md`). Each one cites `tickets/CONVENTIONS.md` by
section and carries word for word the same shared rules and adversarial-review deltas.
The double is derived from the specification, read with the maintainer's four rulings in
`double/SPEC-NOTES.md`. The Python and Rust sources are never read for behaviour. It has
the Python double's coverage: 93 scenarios, 32 rules fired and 35 invariant controls. It
runs through its own recipe, `just gleam-double`, outside `just check`.

## Is one actor right?

Yes, with one refinement. The specification's Lease expiry comment says the
implementation "applies the lapse and every action from one serialized stream (decision
0019)", and decision 0019 rejects partitioning by pocket key, because positions and
`PullClaimsFrontPocket` need one linear order. So one owner process is faithful, and a
process per pocket, per lease or per consumer would be wrong.

The refinement: the actor holds only the serialization. The rules live in a pure core
the actor calls, and `Tick` is an ordinary message in the same mailbox, not a timer per
lease.

## Before GD01 starts

- **One branch.** Every ticket's `branch:` is `build-gleam-double`, and the series runs
  in this one worktree, two commits per ticket, as PD01 to PD06 ran at the maintainer's
  direction. That departs from `tickets/CONVENTIONS.md` §11 and from `AGENTS.md`'s one
  ticket per worktree. Say so before GD01 starts if each ticket should have its own
  `ticket/gd0N-*` worktree instead.
- **The base.** This branch stacks on `build-python-double`, which is not on `main`.
- **Commit these files first.** An untracked file does not appear in a new worktree, and
  each ticket's baseline is quoted at a commit that holds its ticket.
- **The paths are a proposal.** The tickets put the double under `gleam-double/`, the
  package `pachinko_double`, and the spec notes at `gleam-double/SPEC-NOTES.md`. A rename
  applies to all seven tickets.
- **What may be read.** `double/SPEC-NOTES.md` is input. `double/pachinko_double/` and
  `double/tests/` are never read, and `double/README.md` only in GD07, for the shape of a
  guide. The second of those is this series' reading of "spec plus rulings": the
  scenarios are ported from the Rust tests, as the Python double's were, so that the
  Python tests' own choices are not copied. Shared rule 1 carries it; change it there, in
  all seven tickets, if the Python tests should be readable.
- **`BiscuitBin/knarr` was read late.** The plan these tickets implement was drafted
  from documentation, without opening knarr. It was read on 2026-11-20 at commit
  `d7cf7b41416446f1924f144702b74c385471561d`, before the tickets were written, and "Where
  this departs from knarr" says what it changed.

## Design fixed before GD01

A design pass fixed these points before any Gleam exists. Shared rules 3, 4 and 6 carry
them, and GD01's design table lays them out. Nothing here has been run: GD01 proves each
toolchain fact red first, on the thinnest real path, before GD02 builds on it.

- **Layout.** A sibling `gleam-double/`, package `pachinko_double`: rule modules named
  for the specification's sections, `core` (the apply step), `owner` (the only module
  that imports `gleam/erlang` or `gleam/otp`), `SPEC-NOTES.md`, and tests under `test/`.
- **A pure core.** Rule rows are `(name, guard, effect)`, in one table per trigger.
  Dispatch evaluates every guard. A refusal, a guard overlap, a defect and a broken
  invariant are `Result` values, never panics, because a panic would kill the owner. The
  apply step returns the names of the rules that fired. A guard never reads the clock, so
  a refusal cannot; the expiry and decay guards compare against the `now` their `Tick`
  read. GD01's design checkpoint settles how an effect asks for `now` and for a lease id;
  the proposal is a `Step` continuation, with a clock function as the fallback.
- **One owner actor.** One `gleam_otp` actor holds the state and the injected clock and
  lease-id functions. It takes one message per trigger, plus `Tick`, `Inspect` and
  `LeaseView`. On an error it replies and keeps the old state, and a test proves it.
- **The test runner.** No gleeunit: EUnit fixes no order and halts the VM, so nothing can
  run last. The test module's own `main` runs a typed registry, `Test(name, tags, run)`,
  one monitored process per test, then the coverage check. Tags are data, so a tag cannot
  sit on a test that never ran.
- **Coverage.** The same three sources and counts as the Python double's coverage test,
  parsed with line prefixes and no regular-expression package, with `pending_scenarios`
  and `pending_rules` shrinking ticket by ticket.
- **The toolchain.** `gleam` 1.19.1 and `erlang` 29.1.2 from conda-forge through pixi,
  pinned exactly as decision 0011 pins a tool, in a `gleam` feature with its own
  environment. The hex packages `gleam_stdlib`,
  `gleam_erlang`, `gleam_otp` and `simplifile`, as ranges in `gleam.toml`, pinned by a
  committed `manifest.toml`. Decision 0021, which GD01 writes, records all of it, since
  decision 0007's note is Cargo-specific.
- **The recipes.** `just gleam-double` checks the format, builds with warnings as errors
  and runs the tests. It sits outside `just check`, like `just python-double`, and never
  reaches the network. Two steps do, each a named, authorised step in GD01: `just lock`
  for the pixi pins, and `just gleam-double-deps` for the `gleam` environment and the hex
  packages.
- **Keeping `just check` green.** `gleam-double/build/packages/` will hold the
  dependencies' own TOML, Markdown and sources. `.gitignore` gains `/gleam-double/build/`
  and `erl_crash.dump`, which keeps the hook gate out, since it reads tracked files and
  untracked files Git does not ignore. The gates that walk the filesystem by glob,
  `taplo.toml` and `.markdownlint-cli2.jsonc`, are expected to need their own entries,
  and `manifest.toml`'s checksums an entry in `_typos.toml`. GD01 adds only what a red
  run asks for, and its acceptance criteria require `just check` green with `build/`
  populated.
- **GD01 touches more than PD01 did:** `docs/decisions/` (0021 and its index),
  `docs/manifest.yml`, `pyproject.toml`, `pixi.lock`, `AGENTS.md`'s list of network
  recipes, `docs/reference/commands.md` and `docs/project/repository-map.md`. The Python
  series' "no change under `docs/decisions/`" non-goal is dropped for GD01 only.

## Where this departs from knarr

`BiscuitBin/knarr` is the maintainer's Gleam project and the nearest house precedent.
What the tickets took from it, and where they knowingly differ:

| Point | knarr | These tickets | Why |
| --- | --- | --- | --- |
| The pins | `gleam` 1.19.0 and `erlang` 29.1.2 through pixi | `gleam` 1.19.1 and `erlang` 29.1.2 | conda-forge's newest on 2026-11-19. knarr proves 1.19.0 on that Erlang, so 1.19.0 is GD01's first fallback |
| A drifted `manifest.toml` | A pre-check, `manifest-check`, and a `cksum` guard around every `gleam` recipe | The `cksum` guard, in both recipes | Taken from knarr: the plan had no guard. GD01 decides whether the pre-check is needed too |
| Offline runs | `just check` passes with the network denied, from the package cache | The same claim for `just gleam-double`, proved in GD01 with knarr's form of the probe | Taken from knarr |
| The test runner | gleeunit, with qcheck and birdie | The double's own runner, and no other test package | The coverage check must run after every test and read what each one fired, which EUnit's order and gleeunit's halt rule out |
| A linter | glinter, inside the gate | None | The double is outside `just check`; `gleam build --warnings-as-errors` is its only static check. Decision 0021 can add one |
| `rebar3` | Checksum-pinned, for `prometheus` | None | No package the double takes is built with it. GD01 confirms that |
| Supervision | Every process is supervised | No supervisor: a test starts the owner bare | The double is not an OTP application. knarr's finding that a bare actor ignores `gen_server:stop` is carried into GD01 |
| FFI | Confined to named `*_ffi.erl` modules | None under `src/`, and none under `test/` unless the runner needs it | Shared rule 4 |
| Effects | Sans-IO: pure builders and decoders, the sending function injected | The same idea: a pure core, with the clock and lease-id functions injected into the owner | Decisions 0003 and 0008 here; knarr's decision 0002 |

## Index

The `status:` field in each ticket's frontmatter is authoritative; this table is a
snapshot.

| Id | Title | File | Blocked by | Status |
| --- | --- | --- | --- | --- |
| GD01 | The toolchain, the owner actor and the joins | `GD01-toolchain-owner-and-joins.md` | none | done |
| GD02 | The model, the invariant checker, coverage and publish | `GD02-model-checker-coverage-publish.md` | GD01 | done |
| GD03 | Pull and the lease | `GD03-pull-and-the-lease.md` | GD02 | todo |
| GD04 | Ack, nack, release and the lease's end | `GD04-ack-nack-release.md` | GD03 | todo |
| GD05 | Deferral behind the held version | `GD05-deferral.md` | GD04 | todo |
| GD06 | The clock, with Tick, lapse, parking and decay | `GD06-clock-tick-lapse-parking-decay.md` | GD05 | todo |
| GD07 | Close-out and the reader's guide | `GD07-close-out.md` | GD06 | todo |

## Dependencies

The series is a chain, so no two tickets run in parallel.

```text
GD01 ──> GD02 ──> GD03 ──> GD04 ──> GD05 ──> GD06 ──> GD07
```

## What each ticket delivers

| Ticket | Size | What works when it is done |
| --- | --- | --- |
| GD01 | L | `just gleam-double` runs offline and green. A publisher and a consumer join through the owner actor, and a bad config is refused |
| GD02 | L | A publish through the owner creates, supersedes or sheds, with the checker run after every outcome and the coverage check run after every test |
| GD03 | M | A consumer pulls, holds, pulls again and renews. Callers racing for the front pocket get exactly one lease |
| GD04 | L | Marks, fencing, release, the `Freed` rows with nothing deferred, dead letters, the idle revival and the lease view |
| GD05 | M | A newer version of a held key is deferred and never lost, and the release reconciles all six `Freed` rows |
| GD06 | L | `Tick` is a mailbox message. A late action ahead of the tick succeeds (decision 0019's branch A), and nothing is pending |
| GD07 | M | Strict coverage, the reader's guide with a checked walkthrough, and the findings: each `G`-note against its `N`-note, and what the actor showed |

## What each ticket owns

Each of the 93 catalog scenarios and each of the 32 rules is owned by exactly one ticket,
in the ownership table under that ticket's Goal. The split is the Python series' final
one, with its four moves already applied (S037, S080, S088 and S091), except that PD01's
share is divided between GD01 and GD02. GD02 owns all 35 invariants, because the checker
lands whole before any publish scenario is ported.

| Ticket | Rules | Invariants | Scenarios | Scenarios still pending on exit |
| --- | --- | --- | --- | --- |
| GD01 | 2 | 0 | 5 | 88, though the lists do not exist until GD02 |
| GD02 | 4 | 35 | 6 | 82 |
| GD03 | 5 | 0 | 18 | 64 |
| GD04 | 10 | 0 | 18 | 46 |
| GD05 | 4 | 0 | 12 | 34 |
| GD06 | 7 | 0 | 34 | 0 |
| GD07 | 0 | 0 | 0 | none; the pending lists are deleted |

Why GD01 and GD02 are split where PD01 was one: no Gleam toolchain has run in this
repository, so GD01 proves it on the thinnest real path before the model and the checker
are built on top. The seam between them is the snapshot. GD01 lands it for what GD01
holds, `next_position`, the publishers and the consumers, so that its refusals are proved
by comparing snapshots. GD02 completes it and lands the checker and the coverage check
over it. GD01's tests carry their scenario tags as data from the start, and GD02's
coverage check is the first thing to read them.

## What the actor adds to the Python double's tests

No row of the catalog names competing callers. Three tickets add untagged tests in which
several processes send at once, each asserting only what holds in every mailbox order:

| Ticket | Competing callers |
| --- | --- |
| GD01 | The same join, sent by several processes |
| GD03 | Several consumers pulling for one pocket, and for as many pockets as consumers |
| GD06 | An action after `expires_at` and the `Tick` that would lapse its lease |

GD07 writes up what these showed about the specification's "one serialized stream", and
what tests that fix no order cannot show.

## Adversarial review

Every ticket runs `tickets/CONVENTIONS.md` §14 before its final commit, with Codex
(Astra, `gpt-6-astra`) as the reviewer through subagents. Separate subagents triage the
findings and apply the valid fixes. GD01 adds a checkpoint after its design, and GD02
one after its model and checker, each before any scenario of its own is ported. GD07's
checkpoint reviews the whole double and its spec notes. Those three add a `design` lens.
Until T37 lands, every ticket embeds the corrected Codex command, which closes stdin and
keeps the findings apart from `-o`'s last message.

The deltas are the Python series', with three changes of wording: the design lens asks
about modules and about what holds queue state, where the Python series' asked about
classes; the recipe is `just gleam-double`; and `base` is the ticket's base commit, since
the series stacks on one branch.

## Open points

- **PD07 is not a blocker.** `.scratch/python-double/PD07-inspection-contract.md` asks
  whether the inspection shape is part of the contract. Its answer may change what the
  snapshot lists and how S053, S066, S075, S091 and S093 are ported. The Gleam double
  follows the rulings as they stand, and GD02, GD04, GD05, GD06 and GD07 each carry the
  point.
- **Unverified until GD01 runs:** that conda's `erlang` 29.1.2 runs `gleam` 1.19.1, that
  a populated `gleam-double/build/packages/` keeps every later run off the network, and
  that the hook gate and the glob gates stay green beside it. GD01 carries each as a
  red-first probe with a stated fallback.
- **`AGENTS.md`.** GD01 adds `just gleam-double-deps` to its list of recipes that reach
  the network. The Python series left `AGENTS.md` alone; the maintainer approved this
  addition on 2026-11-20.
- **A separate pixi feature.** Settled by the maintainer on 2026-11-20: `gleam` and
  `erlang` sit in a `gleam` feature with its own environment, with
  `no-default-feature = true`, so the default environment, which every CI job installs,
  is unchanged. `just gleam-double-deps` installs that environment, and both recipes put
  its `bin` on their own `PATH`. GD01 was amended before its step 1, and its decision
  0021 records the choice.
- **Renovate and the hex pins.** Decision 0021 must say whether Renovate reads
  `gleam-double/gleam.toml` once it reaches `main`, and whether anything audits a hex
  package.
