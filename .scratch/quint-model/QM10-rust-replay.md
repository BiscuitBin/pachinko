---
id: QM10
title: "Conformance: the Rust replays the model's traces"
status: todo
depends_on: [QM07]
parallel_with: [QM08, QM09, QM13]
branch: ticket/qm10-rust-replay
estimated_size: M
---

# QM10: Conformance: the Rust replays the model's traces

## Context

After QM07 the model in `quint/pachinko.qnt` holds every rule of
`docs/specs/pachinko.allium` but the lapse, the clock and the two decays, and
`just verify-quint` checks it at the walk's domains. Nothing yet ties the model to the
Rust. The proptest walk in `crates/libpachinko/tests/queue/walk.rs` checks the queue
against a model written in Rust beside it (`crates/libpachinko/tests/queue/walk/model.rs`),
which is a third derivation of the Allium, not the Quint model. Ruling 6 of
`.scratch/quint-model/README.md` asks for the tie: the Rust replays the model's traces.

This ticket adds the step relation the traces come from, the committed traces, a Rust
test that replays them, and the two checks that keep the traces fresh. It owns no rule,
invariant or guarantee.

What exists, read at commit `cf1f3f3` and to be read again at the ticket's base:

- **The walk's vocabulary.** `WalkStep` (`walk.rs` lines 82 to 117) has `PublisherJoins`,
  `ConsumerJoins`, `Publish`, `Pull`, `Ack`, `Nack`, `Renew`, `Release`, `Advance` and
  `Tick`. Each index picks from `PUBLISHERS`, `CONSUMERS`, `POCKETS` and `KEYS` (lines 38
  to 43). A publish's payload is `format!("{publisher}{pocket}{key}{rank}")` over the
  three indices and the rank (line 307). `config()` (lines 59 to 67) is the walk's
  configuration.
- **The walk's start.** `walk` (lines 642 to 663) sets the clock to `START`, 1,000 ms,
  and runs `PublisherJoins(0)`, `ConsumerJoins(0)` and `ConsumerJoins(1)` (lines 653 to
  655) before any step it was given. So a walk never starts from the empty queue the
  model's `init` describes.
- **The walk's lease choice.** `LeaseChoice` (lines 71 to 78) is `Own`, `Issued(u8)` or
  `Never`. `resolve` (lines 219 to 230) turns `Issued(n)` into
  `issued[n % issued.len()]`, where `issued` (line 175) is every lease id a pull has
  handed out, in order, ended leases included. `Own` is the consumer's active lease,
  whichever that is. The library's ids come from `CountingLeaseIds`
  (`crates/libpachinko/src/lease_ids.rs`), which counts from 1.
- **The walk's step.** `run` (lines 550 to 591) is private to `walk.rs`. It predicts the
  step with the walk's own model, applies it to the queue, asserts the queue's answer
  equals the prediction, and then `finish` (lines 594 to 621) checks the post-state, all
  35 invariants and every transition. It returns nothing: the queue's answer does not
  leave the helper that got it.
- **`Model::of`.** `Model` (`walk/model.rs` lines 71 to 84) holds `next_position`, the
  publishers, each consumer with its active lease, each pocket with its status, position,
  `consecutive_expiries` and records, `pocket_order`, the active leases, and the two shed
  queues as `Vec`s in the snapshot's order, which is expiry and then arrival
  (`crates/libpachinko/src/queue/inspect.rs`). `Model::of` (lines 113 to 157) projects a
  `Snapshot` into it. Its fields are private to `model.rs`. An ended lease is not in it:
  `end` (lines 525 to 529) removes the lease. The walk keeps ended leases apart, in
  `Walk::ended` (`walk.rs` line 178). Timestamps are in it: a lease's `granted_at` and
  `expires_at`, a shed entry's `shed_at` and `expires_at`, a dead letter's `nacked_at` and
  `expires_at`, and a held record's `nacked_at`.
- **Gate 8 covers test code.** `scripts/check-effects.sh` lines 200 to 206 run clippy
  over libpachinko with `--all-targets` and forbid `clippy::disallowed_methods` and
  `clippy::disallowed_types`, and `crates/libpachinko/clippy.toml` bans `std::fs::read`,
  `std::fs::read_dir` and `std::fs::read_to_string` (lines 120 to 123) and
  `std::env::var` (line 161). Read, not run: a test under `crates/libpachinko/tests/`
  that opens a file at run time fails `just effects`, and no `#[expect]` can lift it.
- **Other gates that bound a fixture and its test.** `.pre-commit-config.yaml` lines 92
  and 93 refuse any file over 768 KB, and its `check-json` hook parses every `.json`
  file. `.editorconfig` asks for a final newline, no trailing space and indentation in
  steps of two. `rustqual.toml`, under `[tests]`, holds a test file to 1,000 code lines
  and a test function to 100. Coverage is measured over `crates/*/src/**`, so a test
  adds to it and cannot lower it.
- **`serde_json` is already a workspace dependency.** `Cargo.toml` lists it with a
  decision-0007 note for `crates/pachinko/tests/http.rs`. `crates/libpachinko/Cargo.toml`
  has two development dependencies, `divan` and `proptest`, and no JSON reader.
- **The CLI's `run`.** Read in
  `tools/quint/node_modules/@informalsystems/quint/dist/src/cli.js`, not run: `--main`,
  `--init` (line 264), `--step` (line 269), `--seed` (line 295, any text `BigInt`
  accepts), `--max-samples` (line 245, 1 by default when a seed is given), `--n-traces`
  (line 249), `--max-steps` (line 254, 20 by default), `--n-threads` (line 259, the
  machine's core count by default), `--out-itf` (line 241), `--mbt` (line 305),
  `--backend` (line 310, `rust` by default), `--no-invariant` (line 52) and
  `--observation` (line 57). `cliHelpers.js` lines 192 to 204 put `description`, built
  from `new Date()`, and `timestamp`, from `Date.now()`, into every ITF file's `#meta`,
  beside `source` and `status`. `itf.js` writes an integer as `{"#bigint": "<n>"}`, a set
  as `{"#set": [...]}`, a map as `{"#map": [[k, v], ...]}`, a tuple as `{"#tup": [...]}`,
  a variant as `{"tag": ..., "value": ...}`, and gives each state `"#meta": {"index": i}`.

The three mismatches the mapping bridges:

1. **The start.** The model's `init` is empty and the walk starts after three joins.
2. **Lease identity.** The model names a lease its own way (QM02 settles how). The Rust
   names it by a `LeaseId` the lease-id source chose. The walk's `Own` cannot name an
   ended lease, and a model step can.
3. **Refusals.** A model step that no rule answers changes nothing and leaves `fired`
   empty. The Rust answers the same step with a named error. `tickets/T43-caller-contract.md`
   is to rule on those names and their order.

QM08 and this ticket are a parallel pair (README, "Dependencies", "Parallel pairs"),
and QM09, which follows QM08, may also land on either side of this one. QM08 adds
`lapse(lease)` to `step` and never to `replayStep`, and QM09 adds the clock's actions to
`step` and nothing to `replayStep`, so no fixture holds a lapse, an advance or a tick
whichever lands first. Whichever lands second rebases, runs `just quint-traces` before
any gate because the model's bytes changed, keeps the replay green as shared rule 11
asks, and reruns its verification. QM09 also adds a second instance,
`pachinko_zero_ttl`. The fixtures come from `pachinko_walk` only, whatever instances
the model has.

`tickets/T43-caller-contract.md` (answers, the lease's id, the named refusals) and
`tickets/T45-drop-joining.md` (the start of every fixture) change this ticket's mapping
when they land. Each carries the obligation to update it (README, "The T tickets that
touch the model").

Read these:

- `.scratch/quint-model/README.md`, in full, and the tickets QM02 to QM07 under
  `.scratch/quint-model/` for the model's state, the ghosts and `witness_<Rule>`.
- `docs/specs/pachinko.allium`: the comments above "Publish", "Pull", "Ack and nack",
  "Renew and release" and "Lease end".
- `crates/libpachinko/tests/queue/walk.rs`, `walk/model.rs`, `invariants.rs`, `main.rs`
  and `fixture.rs`; `crates/libpachinko/Cargo.toml` and `Cargo.toml`.
- `docs/reference/testing.md`; `docs/decisions/0007-dependency-policy.md` and
  `docs/decisions/0009-rust-quality-gate.md`; `AGENTS.md`, invariants 2, 8 and 9.
- `rustqual.toml`; `Justfile`: `test`, `coverage`, `metrics`, `effects`, `lock`,
  `lock-check`, `deny`, `deps-unused`, `quint`, and QM02's `check-quint`;
  `scripts/check-effects.sh`; QM02's `scripts/check-quint.sh` and QM03's
  `scripts/verify-quint.sh`.
- The CLI source named above: `cli.js`, `cliCommands.js` (`runSimulator`, from line 280),
  `cliHelpers.js`, `cliReporting.js` (`prepareOnTrace`, from line 327) and `itf.js`.
- `tickets/CONVENTIONS.md` §11 and §14.

This ticket changes Rust under `crates/`, so `AGENTS.md`'s block for that applies to the
agent that runs it: use the `rust-change` skill, and `review-change` to review the
result. Use `review-docs` for the pages and `plan-change` to pick the ticket up.

## Goal

`quint/traces/` holds traces of `pachinko_walk`, generated by `just quint run` over
`replayStep` at a fixed seed and kept in a normalised form that two runs write byte for
byte alike. A test in libpachinko's queue suite reads them, maps each model step to a
`WalkStep`, applies it to a real `Queue` through the walk's own `run`, and compares, after
every step, the queue's state with the model's through `Model::of` and the queue's answer
with the ghost `answer`. It runs offline inside `just test`. `just check-quint` fails,
offline, when `quint/pachinko.qnt` no longer has the hash the traces were generated from.
`just verify-quint` generates the traces again and fails on any difference. One command,
`just quint-traces`, regenerates everything after a model change.

What the replay shows is that the Rust agrees with the model on the traces committed,
no more. It is sampling, as the walk is. The handbook says so.

### Owns

| Kind | Names |
| --- | --- |
| Rules | none |
| Invariants | none |
| Guarantees | none |
| Partial or extended, not counted | Adds `replayStep` and `replayInit` to the model. They restrict `step` and `init` and add no rule, no property and no state |

## Design proposed in this ticket

These are proposals. The maintainer confirms or changes them at the **Stop** in step 3,
with checkpoint 1's findings in hand, and the hand-back records the ruling. After that the
builder does not reopen them. The README's design point for QM10 has five parts, "The
fixture format, the seed, how the model's hash is recorded, how a trace starts where the
walk starts, and how the replay lists what it does not compare". The first four bullets
answer the first four, and the bullet "One list of what the replay does not compare"
answers the fifth.

- **The fixture format: a normalised form of ITF, in one file.** Recommended:
  `quint/traces/pachinko_walk.json`, one JSON document of the form
  `{"format": "pachinko-trace/1", "vars": [...], "traces": [{"name": ..., "seed": ...,
  "states": [...]}]}`. Each state is the ITF state `--out-itf` wrote, changed in exactly
  these ways, and the script fails on anything it does not expect:
  1. the trace's `#meta` is dropped, after its `status` is checked (`ok` for a plain
     walk, `observed` for a witness trace). That removes `description` and `timestamp`,
     the wall-clock fields, and `source`, a path;
  2. each state's `#meta` is dropped; a state's index is its place in `states`;
  3. the ghost `prev` is dropped, after checking that `prev` in state `i` equals the
     state variables of state `i - 1`. It doubles the file and says nothing new;
  4. `{"#bigint": "<n>"}` becomes the JSON number `n`, and the script fails on a value
     outside 53 bits;
  5. the elements of every `#set` and the entries of every `#map` are sorted by their
     serialised text, so the file does not depend on the evaluator's iteration order;
  6. keys are written in a fixed order, with compact separators, one state to a line and
     a final newline, so `.editorconfig` holds and a diff shows the states that changed.

  One file, because the test cannot list a directory (Context, "Gate 8 covers test
  code") and reads the fixture with `include_str!`. The file stays under the hook's 768
  KB; a set that does not fit is a **Stop**, never a raised limit. Alternative: raw ITF,
  one file per trace, with only the two wall-clock fields removed. It is readable by any
  ITF tool, but the test then needs one `include_str!` line per trace kept in step with
  the generator by hand, and the files are several times larger. Step 2 measures both.
- **The seed, and the number and length of traces.** Recommended: one seed, `1`, recorded
  in the manifest. The set has two parts.
  - *A witness trace for every rule a replay can reach.* For each of the 25 rules QM02
    and QM04 to QM07 own, one run with `--observation=witness_<Rule>`, `--seed=1`,
    `--max-steps=12` and `--max-samples=20000`. Read, not run (`cliCommands.js` lines 315
    to 323, 373, 391 and 425): such a run writes only a trace that reaches the
    observation, with status `observed`, and exits non-zero when none does. So the set
    fires every one of the 25 rules by construction, and a rule that cannot be reached
    stops the generation. The seven rules left out need a lapse or the clock:
    `LeaseLapses`, `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`,
    `LeaseExpiryIdlesEmptyPocket`, `ReviveParkedPocket`, `SupersededRecordDecays` and
    `DeadLetterDecays`.
  - *Four plain walks* of 30 steps, at seeds 1 to 4 (the seed, plus the walk's index),
    each one run with `--max-samples=1`.

  A witness trace's `name` is `witness_<Rule>` and a plain walk's is `walk_<seed>`, so
  the test can ask for the trace that was generated for a rule and not only for a rule
  that some trace happens to fire.

  Every run passes `--main=pachinko_walk --init=replayInit --step=replayStep
  --backend=rust --n-threads=1 --no-invariant --n-traces=1`. `pachinko_walk` is the only
  instance a fixture comes from, however many instances the model has. One run per trace,
  because which traces `--n-traces` keeps out of several samples is not read. Alternative: plain
  walks only, 16 of 30 steps, with the rules they fire reported and not required. It is
  simpler, and a uniform random walk of that length is unlikely to reach the rows of the
  lease-end table that need a publish onto a held key followed by a nack and a release,
  so the replay would say little about QM07. The numbers are starting values: step 2
  measures the file's size and the generation's time, and brings both to the **Stop**.
- **How the model's hash is recorded.** Recommended: a second file,
  `quint/traces/manifest.txt`, in the form of `tools/quint/apalache.txt`: comment lines
  starting `#`, then `key=value` lines, read with `sed` or split on the first `=`, never
  sourced. Its keys are `format`, `model_sha256` (the SHA-256 of the bytes of
  `quint/pachinko.qnt`, lowercase hex), `fixture_sha256` (the same for
  `quint/traces/pachinko_walk.json`), `quint` (the CLI version `tools/quint/package.json`
  pins), `evaluator` (the version `tools/quint/evaluator.txt` pins, created by QM01),
  `main`, `init`, `step`, `seed`, `walks`, `walk_steps`, `witnesses` (the rule names,
  space separated), `witness_steps`, `witness_samples` and `traces` (the count). The hash
  is over bytes, so a comment-only edit to the model also fails the gate until the traces
  are regenerated; that is the price of a check that needs no parser. Alternative: the
  same keys as a header object inside the fixture. One file fewer, but `sh` cannot read
  it and the file's own hash cannot be in it.
- **How a trace starts where the walk starts.** Recommended: the model gains `replayInit`,
  the state `init` reaches after `publisherJoins("p0")`, `consumerJoins("c0")` and
  `consumerJoins("c1")`, with `fired` empty and `last` as `Init`. It is built from
  `init`'s state through the same definitions the two join actions use, never as a second
  literal, so it cannot drift from them. The Rust test runs the walk's own three joins
  and then compares the queue with state 0 of every trace, which is what shows
  `replayInit` is that state. `p1` and `c2` still join only if a step says so.
  Alternative: start from `init` and check that every trace begins with the three join
  steps. That spends three of every trace's steps and needs a generator that can force
  them. T45 removes joining and with it `replayInit`.
- **`replayStep`.** It offers exactly the top-level actions of `step` that have a
  `WalkStep` and that the model has when this ticket runs without a clock:
  `publisherJoins`, `consumerJoins`, `publish`, `pull`, `ack`, `nack`, `renew` and
  `release`, over the same arguments `step` gives them. It never offers `lapse`, so a
  lapse cannot appear in a fixture whether or not QM08 has merged. If QM09 has merged, it
  does not offer `advance`, `tick` or QM09's model-only decay steps either: QM11 adds
  the first two, and the others are never replayed. As a second guard the generator
  fails on any `last` that is not one of the eight, or `Init` in state 0.
- **One command, three modes.** `just quint-traces` runs `scripts/quint-traces.py`, which
  uses Python's standard library only, as `scripts/diagrams.py` does, and calls Quint
  only as `just quint run`.
  - `just quint-traces` (mode `write`) generates every trace into a new directory under
    `ai_tmp/`, normalises them, and writes the fixture and the manifest. It is the one
    command a model change runs (shared rule 11). It writes tracked files, so it is
    outside `just check`.
  - `just quint-traces check` is offline and read-only and starts no Quint. It fails
    unless the manifest has exactly its keys, both hashes match the files in the
    checkout, `traces` is at least 1 and equals `walks` plus the number of `witnesses`,
    and the fixture, parsed as JSON, holds exactly the traces the manifest names, each
    once and with at least one step after its first state: `witness_<Rule>` for each
    rule in `witnesses`, and `walk_<seed>` for each of the `walks` seeds from `seed`
    up. A second `walk_1` in place of `walk_4` keeps the count and fails here. The hashes
    alone do not show that: a fixture emptied by hand with its hash refreshed matches
    them. Its message names `quint/pachinko.qnt` and `just quint-traces`. QM02's
    `scripts/check-quint.sh` runs it, after its typecheck, with pixi's Python as that
    script already reads JSON.
  - `just quint-traces diff` generates into scratch as `write` does, compares the result
    with the committed fixture and manifest byte for byte, prints a diff and exits
    non-zero on any difference, and writes no tracked file. `scripts/verify-quint.sh`
    runs it as its last step, after every instance, witness and mutant, so a stale
    fixture is reported beside the model's own results and never in place of them. A CLI
    or evaluator bump changes `quint=` or `evaluator=`, so it fails there until the
    traces are regenerated.
- **Where the test lives, and how it finds the fixture.** Recommended: a new module
  `crates/libpachinko/tests/queue/walk/replay.rs`, a child of `walk`, so it can call the
  private `run` and read `Walk`'s private fields, and
  `crates/libpachinko/tests/queue/walk/model/trace.rs`, a child of `model`, which builds
  a `Model` from a decoded state and so needs `Model`'s private fields. The fixture and
  the manifest are read at compile time:
  `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../quint/traces/pachinko_walk.json"))`.
  No `std::fs` and no `std::env::var` (Context, "Gate 8 covers test code"). The test
  fails if the fixture holds no trace, or a number of traces other than the manifest's
  `traces`. Read, not run: `cargo package` copies only files under the crate's
  directory, so a packaged libpachinko could not compile this test target. Both crates
  are `publish = false` (`Cargo.toml`, `[workspace.package]`), and decision 0022 records
  the limit as something a decision to publish must settle. Alternative: a sibling
  `tests/queue/replay.rs` that drives the queue through its public API and never calls
  `run`. It touches `walk.rs` less, and it loses the walk's own prediction, invariants
  and transition checks on the model's traces.
- **`run` hands back the queue's answer.** `walk.rs` changes in three ways and no other:
  `walk`'s prologue (set the clock, the three joins) becomes a function the replay calls
  too; `run` returns what the queue answered, as a small enum: a batch with its lease id,
  pocket key, `expires_at` and records in order, `NoPocketAvailable`, accepted with the
  success value's name and, for a renew, the instant it returned, or refused; and
  `mod replay;` is declared. The proptest walk ignores the return value and behaves as
  before.
- **The mapping reads `last` and `answer`, never `--mbt`.** The step and its arguments
  come from the ghost `last` in state `i`. What the step handed back comes from `answer`
  in state `i`. The generator does not pass `--mbt` (README claim 15).

  | The step `last` names | `WalkStep` | Argument mapping |
  | --- | --- | --- |
  | `publisherJoins(publisher)` | `PublisherJoins(n)` | `n` is the name's index in `PUBLISHERS` |
  | `consumerJoins(consumer)` | `ConsumerJoins(n)` | the index in `CONSUMERS` |
  | `publish` | `Publish { publisher, pocket, key, rank }` | indices in `PUBLISHERS`, `POCKETS` and `KEYS`; the rank as it is |
  | `pull` | `Pull { consumer, limit }` | the limit as it is |
  | `ack`, `nack` | `Ack` or `Nack { consumer, lease, key }` | the lease through the table below |
  | `renew`, `release` | `Renew` or `Release { consumer, lease }` | the lease through the table below |

  `last` is a value of the model's sum type `Step`, one variant per action and `Init`;
  the test decodes the variant's tag and fails on one it does not know.

  A name outside its domain, a rank outside 0 to 3 or a limit outside -1 to 3 fails the
  test. A payload in the fixture is the publish's tuple; the test turns it into the
  walk's bytes with the same `format!` the walk uses, moved into one function both call.
- **Lease identity: a table per trace, kept after a lease ends.** The replay keeps, for
  each trace, a list of pairs: the model's lease and the `LeaseId` the Rust returned for
  it, in the order the Rust issued them. A pull whose `answer` is a batch under a lease
  not yet in the list appends the pair, and the walk has already asserted that the id was
  never issued before (`walk.rs` lines 372 to 375). A model step that names the lease at
  place `i` maps to `LeaseChoice::Issued(i)`, which `resolve` turns into exactly that id,
  active or ended, because the list and `Walk::issued` grow together; the test asserts
  the two have the same length and that `i` fits a `u8`. A step that names a lease no
  model step has created maps to `LeaseChoice::Never`. `LeaseChoice::Own` is never used:
  it would turn a step that names an ended lease into one that names the consumer's
  current lease. A lease id is compared only through the table, never by value. QM02
  recommends that the model's leases are a list in grant order, a lease's identity its
  index, and the one index past the end the lease never issued. If that was ruled, the
  model's lease at place `i` is index `i`, and the test asserts it for every pair it
  appends, so the table is checked and not assumed. If QM02 was ruled otherwise, the
  table still holds: it pairs whatever the model calls a lease with the Rust's id.
- **What is compared after every step.**
  1. *Accepted or refused.* `fired` is empty exactly when the Rust returned an error.
     The error's name is not compared until T43's tickets land.
  2. *The answer.* A batch: the lease through the table, the pocket key, and the records
     in order, each with its key, rank and payload. `NoPocketAvailable` against the
     Rust's `PullOutcome::NoPocketAvailable`. For the other accepted steps the Rust's
     success value is compared with the rule in `fired`: `Created` with
     `PublishIntoNewPocket`, `NewKey` with `PublishNewKey`, `Superseded` with `Supersede`
     or `DeferBehindHeldVersion`, `Stale` with `DropStalePublish`, `PocketFate::Queued`
     with `ReleaseRequeuesPocket` and `PocketFate::Idle` with `ReleaseIdlesEmptyPocket`.
     A snapshot alone cannot tell two steps that both change nothing apart; this can.
     The Rust's batch also carries the lease's `expires_at`, and a renew returns one
     (`PullOutcome::Batch` in `crates/libpachinko/src/queue/types.rs`, `renew` in
     `crates/libpachinko/src/queue/renew.rs`). The model's `answer` holds a lease, a
     pocket and records, and no expiry (README, second design block). Recommended: the
     replay compares each returned expiry with the list's constant here, and from QM11
     on with that lease's `expires_at` in the fixture's state after the step, so a
     returned value that differs from the stored one fails. Alternative: `answer`
     carries the expiry. That changes a ghost QM02 owns and the README defines, so it
     is the maintainer's at the **Stop**, and not this ticket's edit.
  3. *The state.* `assert_eq!(Model::of(&queue.snapshot()), expected)`, where `expected`
     is built from the fixture's state by `walk/model/trace.rs`: `next_position`, the
     publishers, each consumer's active lease, each pocket's status, position and
     records (key, rank, payload, position, and for a held record the held rank and
     payload, the mark and the deferred position), the active leases with their pocket
     and consumer, and both shed queues entry by entry in the model's list order.
  4. *A lease's end.* The walk's model drops an ended lease, so the replay compares the
     model's ended leases with `Walk::ended`: the same leases through the table, each
     with its pocket, its consumer and `released` as its status.
- **A mismatch names its comparison.** Every assertion of the replay fails with a
  message that begins `replay <trace> step <index>: <comparison>:`. The comparison is
  `start`, `refused`, `answer`, `state`, `ended`, `lease`, `list` (a key or a step that
  is neither compared nor listed) or `count` (the number of traces, at step 0). The
  rule test's failures begin `replay rules:`. Step 7's controls and the Verification
  script read these prefixes, so a run of `just test` that failed anywhere else is not
  counted as the replay failing.
- **One list of what the replay does not compare.** It is one table in `replay.rs`, and
  it is the list shared rule 11 names: every field of a fixture's state and every step a
  fixture holds is either compared by the replay or in the list, with the ticket that
  will map it, and a step is listed only with the maintainer's answer on which ticket
  that is. The test enforces it both ways. `walk/model/trace.rs` fails on a key in a
  state that it neither reads nor finds in the list, at any depth, and on a variant of
  `Step` in `last` that the mapping does not know. And every field of `Model` and of an
  ended lease is either built from the fixture or filled from the list. The list has
  two kinds of row.
  - *What the Rust holds and the fixture does not.* No step of `replayStep` moves the
    clock, so every time the Rust stamps is a known constant, and the replay expects
    that constant instead of skipping the field.
  - *What the fixture holds and the replay does not compare with it.* None at QM07. A
    later model change adds its rows here.

  | Not compared with the fixture | Expected of the Rust instead | Mapped by |
  | --- | --- | --- |
  | A lease's `granted_at`, and an ended lease's `ended_at` | `START` | QM11 |
  | A lease's `expires_at`, a batch's and a renew's | `START` plus `lease_ttl` | QM11 |
  | A shed entry's `shed_at` and `expires_at` | `START`, and `START` plus `superseded_ttl` | QM11 |
  | A held record's and a dead letter's `nacked_at`; a dead letter's `expires_at` | `START`, and `START` plus `dead_letter_ttl` | QM11 |
  | A pocket's `consecutive_expiries` | 0 | the change that gives the model the field, or the ticket that change names |
  | `pocket_order` | the fixture's queued pockets, smallest position first | the tickets T40 creates |
  | A refusal's name | not compared | the tickets T43 creates |
  | A `LeaseId`'s value | through the table only | the tickets T43 creates |

  Who edits the list after this ticket. A ticket that changes the model on a tree that
  holds `quint/traces/`, at its base or after its closing merge or rebase, edits the
  list in its own change, because shared rule 11 makes the replay part of that change:
  it either maps what the model gained or adds a row naming the ticket that will. QM08
  does so for `consecutive_expiries`: it reads the field from the fixture and drops
  the row, or keeps the row and names who will. QM09 does so for `now`, every
  timestamp and whatever else the clock adds to the state: those become rows of the
  second kind, naming QM11, and the constants above still hold because `replayStep`
  has no step that moves the clock. QM11 then maps what QM09 listed. In the other
  order, the model this ticket lands on already has the field, at the base or after
  this ticket's closing merge or rebase: this ticket reads `consecutive_expiries` from
  the fixture and has no row for it, and it writes the clock's rows itself, naming
  QM11. Step 10 looks again after the merge.
  Alternative: leave each such field out of the comparison, with no expected value. It
  is weaker and needs a second projection of `Model`.
- **`serde_json` as a development dependency of libpachinko.** `crates/libpachinko/Cargo.toml`
  gains `serde_json = { workspace = true }` under `[dev-dependencies]`. The crate is
  already in `Cargo.toml` and `Cargo.lock`, but libpachinko taking it is new, so the
  decision-0007 note in `Cargo.toml` is extended to say what libpachinko's test reads
  with it and that `std` has no JSON reader, and
  `docs/decisions/0007-dependency-policy.md`, "Harness dependencies", gains the sentence.
  The test walks `serde_json::Value` and derives nothing, so `serde` is not added and the
  library's types stay free of it.

## Claims this ticket checks

Rows 12, 14 and 15 are the README's. The lettered rows are claims this ticket rests on
that nobody has run. An answer that differs from the one below is a design change: stop
and bring it to the maintainer.

| # | Claim | Check |
| --- | --- | --- |
| 12 | The whole of `verify-quint`, with the trace generation this ticket adds to it, finishes inside the CI job's timeout | step 5: time `just quint-traces diff` and the whole recipe, and compare with the `timeout-minutes` QM03 set. Not read |
| 14 | `run --seed` repeats a trace for a fixed seed and CLI, but the ITF output carries wall-clock metadata, so two runs differ byte for byte | step 2: two runs at one seed differ under `cmp` and are equal once normalised; a run at another seed differs once normalised. Read in `cliHelpers.js` lines 192 to 204, not run |
| 15 | ITF holds states and values, not a log of actions with arguments. `--mbt` adds the action's name and the named `nondet` picks, which depend on how the action is written. The ghost `last` is what a replay reads | step 2: one run with `--mbt` and one without; quote `mbt::actionTaken` and `mbt::nondetPicks` for a step that names a lease beside `last` for the same step. Read in `itf.js` lines 27, 28 and 69 to 138, `runtime/impl/builder.js` lines 254 and 255 and `runtime/impl/nondet.js` lines 73 and 74, not run |
| A | A test under `crates/libpachinko/tests/` that calls `std::fs::read_to_string` fails `just effects`, and one that uses `include_str!` with `env!` passes | step 2, in an untracked test file deleted afterwards. Read in `scripts/check-effects.sh` lines 200 to 206, not run |
| B | `run --observation=witness_<Rule>` with a seed and `--n-threads=1` writes one trace, with status `observed`, whose last state has the rule in `fired`, and writes the same trace again for the same seed. It exits non-zero and writes nothing when no trace reaches it | step 2. Read in `cliCommands.js` lines 315 to 323, 373, 391 and 425, not run |
| C | `run` takes `--init=replayInit` and `--step=replayStep` with `--main=pachinko_walk`, and state 0 of the trace is `replayInit`'s | step 2. Read in `cli.js` lines 264 to 272, not run |
| D | The normalised traces are the same on `linux-64` as on `osx-arm64` for the same seed, CLI and evaluator | the CI job `quint`, once a push is authorised, and only if the committed fixture was generated on `osx-arm64`: the job runs on Linux alone, so it compares Linux with whatever was committed. The hand-back quotes `uname -sm` from the machine that ran `just quint-traces`. Not read, and not shown before then: Open points |
| E | The fixture, at the numbers ruled, is under 768 KB | step 2 measures a sample in both formats; step 5 measures the committed file |
| F | Cargo rebuilds the test when the fixture changes, so an edited fixture is never tested from a stale build | step 7's hand-edit control: the test fails without `cargo clean`. Not read |
| G | Over `replayStep`, the Rust refuses a step exactly when no rule answers it in the model | the replay's own assertion on every step of every trace, step 7. Read in `walk/model.rs`, not run against the model |
| H | `serde_json` as a development dependency of libpachinko leaves `just lock-check`, `just effects`, `just deny`, `just deps-unused`, `just features` and `just test-doc-default` green | step 6. Read: `scripts/check-effects.sh` checks the normal dependency tree and leaves development dependencies exempt (its header, lines 5 to 7) |

## Non-goals

- No replay of `lapse`, `advance` or `tick`, and no timestamp read from a fixture: QM11.
- No replay by the Python double: QM12. This ticket keeps the fixture readable by it:
  plain JSON, no Rust type in it.
- No comparison of a refusal's name, and no comparison of a lease id by value: the
  tickets T43 creates.
- No comparison of `fired` with the Rust beyond accepted, refused and the success value.
  The Rust does not report which rules fired.
- No new property, invariant or mutant in the model, and no change to `step`, `init` or
  any rule. `replayStep` is not verified on its own: it is a restriction of `step`.
- No change to the proptest walk's generator, its 512 cases or what it asserts.
- No `serde` on any library type, and no new workspace dependency.
- No change to a hook's limit, a gate's threshold or the walk's domains.

## Shared rules (QM01 to QM13)

1. **The Allium decides.** `docs/specs/pachinko.allium` is the source of truth for
   behaviour (decision 0002). The model is derived from it and from nothing else. Read
   each rule and invariant in the Allium at the ticket's base, never from a ticket's
   summary of it, a page or a chat. No ticket reads the Rust or the double to decide
   what the model does. A ticket may read them for names, for the walk's domains and
   configuration, and, in QM10 to QM12, to map steps and state.
2. **The model never leads.** A counterexample has one of three causes. The model
   transcribes the Allium wrongly: fix the model. The property transcribes the Allium
   wrongly: fix the property, against the Allium's text. The Allium is wrong or silent:
   stop, write the trace and the question under Open points, and ask the maintainer. Never
   weaken an invariant, narrow a domain, drop a rule or add a guard the Allium does not
   have to make a run pass. Never settle either `open question` in the Allium.
3. **Names are fixed.** The model is `quint/pachinko.qnt`, module `pachinko`. An
   instance is a module `pachinko_<bound>` at its foot; the first is `pachinko_walk`.
   The sum type `Rule` has one variant for each Allium rule the model has so far, spelt
   as the Allium spells it.
   A rule has `guard_<Rule>`, `effect_<Rule>` and `witness_<Rule>`. An Allium invariant
   or guarantee is a property under its Allium name; `FiredMatchesGuards` and
   `UnansweredStepChangesNothing` check the labels. The ghosts are `fired`, `last`,
   `answer` and `prev`. A mutant is `quint/mutants/<ticket>-<slug>.patch`. Top-level
   actions carry `WalkStep`'s names (`crates/libpachinko/tests/queue/walk.rs`) in lower
   camel case. The gate is `just check-quint`, the verify recipe is `just verify-quint`,
   and the trace fixtures live in `quint/traces/`.
4. **One action per cascade, and every step is total.** A pull with its `HoldRecord`
   steps is one action. A lease's end with its `Freed` rules is one action. A publish
   with the revival it causes is one action. A step that no rule answers changes
   nothing and leaves `fired` empty; from QM09, `advance` and a `tick` with nothing due
   move only the clock. No guard and no effect on the state reads a ghost.
5. **The domains are a ruling.** `pachinko_walk` has the walk's domains and
   configuration: 2 publishers, 3 consumers, 3 pockets, 3 record keys, ranks 0 to 3,
   pull limits -1 to 3 (-1 and 0 until QM05), `max_pull_batch` 2 and
   `max_consecutive_expiries` 2. A ticket
   that cannot verify at these inside the CI job's timeout stops and asks. It does not
   shrink a domain or drop an instance. QM03 sets each instance's `--max-steps`; a
   later ticket may raise it and never lowers it.
6. **Verified means this.** From QM03 on, `just verify-quint` shows all of it. `Canary`
   is violated on every instance. Every property passes on every instance, each named
   on the command line. Every rule the ticket owns has its witness reached. Every
   mutant the ticket lists typechecks and is caught by the one property it names, which
   may be any property the recipe checks, and which the unchanged model's pass on that
   instance shows to hold at the same bound. Each check fails closed: zero instances, a
   witness not reached, a patch that does not apply, a mutant that survives, a mutant
   caught by another property, and an error or a deadlock read as a catch are all
   failures. QM02 shows the same by hand for one witness and one mutant. An owned
   invariant that no listed mutant aims at is reported in the hand-back as checked and
   not mutation-tested.
7. **No clock before QM09.** QM02 to QM08 add no `now`, no timestamp field and no TTL.
   While the model has no clock, a specification change that only says when something
   is due changes nothing in the model, as long as the untimed lapse still covers it.
8. **What the model lacks is written down.** A behaviour the Allium allows and the
   model does not yet have is one line in the model's header comment, under "Not yet
   modelled", naming the ticket that adds it. That ticket deletes the line. A property
   that would fail only because of such a gap carries the gap as a named exception, and
   the same ticket removes the exception. A property is transcribed whole, conditions
   included, even where a condition is always true so far.
9. **Quint runs through the recipe.** Every Quint command is `just quint <args>`, run
   from the repository's root, never a bare `quint` on `PATH`. A claim about the CLI's
   behaviour is checked by running the pinned CLI and quoting its output, never from
   memory or from a page.
10. **The network is named, and the checker is the pinned one.** `just initialize`,
    `just install-quint`, `just install-apalache` and `just install-quint-evaluator`
    reach the network, and each is a named, authorised step in the ticket that runs it
    (`tickets/CONVENTIONS.md` §11, "Network"). `just check` reaches none. No step lets
    Quint download anything itself. Before any `verify`, show that nothing is listening
    on `localhost:8822`; never pass `--server-endpoint`; never run a `verify`, or any
    other `just quint` command that needs the evaluator, while `just install-apalache`
    or `just install-quint-evaluator` runs or while another worktree verifies. A `verify` talks
    to its own server over the loopback interface, so a network denial used as evidence
    for a `verify` denies the external network and allows loopback, and its negative
    controls are §11's two `curl` commands.
11. **One change carries the model and what depends on it.** A change to
    `quint/pachinko.qnt` re-cuts every mutant patch it breaks, keeping the mutation and
    the property it names, and runs `just verify-quint` once QM03 has landed that recipe.
    Once `quint/traces/` exists it also regenerates the fixtures, before any gate runs,
    because `just check-quint` then compares the model's hash, and keeps both replays
    green: the Rust test in `just test` and, once QM12 has landed, `just python-double`. A
    field or a step the model gains is either mapped in each replay or added to that
    replay's list of what it does not compare, naming the ticket that will map it. A step
    on a replay's list ends that replay of every trace that holds it, so a ticket lists a
    step only with the maintainer's answer on which ticket maps it. The patches, the
    fixtures, their generator and the replays are in the Files touched of every ticket
    that changes the model, and the hand-back lists each patch re-cut. A change to
    `docs/specs/pachinko.allium` that touches a modelled rule, invariant or guarantee
    updates the model in the same change, runs `just check-quint`, and runs
    `just verify-quint` if the recipe exists.
12. **Files.** Edit only the files the ticket lists, plus its own `status:` line and
    hand-back notes. A shared file is listed as `shared, own lines`. A change needed
    elsewhere is handed back, not made (`tickets/CONVENTIONS.md` §11). Scratch goes in
    `ai_tmp/<ticket>/` and is never committed.
13. **Evidence, and the series base.** Each quoted output is headed by the commit it ran
    at and shows its exit status. A check that passes by printing nothing follows its
    positive control (`tickets/CONVENTIONS.md` §11, "Quoted evidence names its commit"
    and "Checks fail closed"). Where §11 says `main`, read the series base the
    maintainer named: for the worktree's start, for the closing fetch and merge, and
    for the evidence quoted afterwards. Pushing and opening a pull request are
    separately authorised.

## Adversarial review

This ticket runs `tickets/CONVENTIONS.md` §14 at each checkpoint it marks: its phases,
its two lenses, its two rounds at most, its scope guard and its record. These deltas
apply, and every ticket of the series carries them word for word.

- **Reviewer.** Codex, Sol, which is `codex exec -m gpt-6.2-sol`. The maintainer named
  Sol for this series on 2026-11-25, in place of the model §14 prints. Confirm
  `codex exec --help` on the day, as §14 asks. Fall back to a fresh-context
  `claude-opus-5-5` subagent only if `codex` is not on `PATH`, and say so in the
  hand-back notes.
- **A usage limit is not a clean review.** If a Codex log ends in a usage-limit error,
  the lens is incomplete. Stop and ask the maintainer whether to wait or to use the
  fallback. Never record it as "no findings".
- **Command.** Run this from the worktree, once per lens:

  ```sh
  dir="$PWD/ai_tmp/review/<id>-<n>"
  mkdir -p "$dir"
  codex exec -m gpt-6.2-sol -s read-only -C "$PWD" \
    -o "$dir/last-<lens>.md" "<brief>" \
    < /dev/null >| "$dir/log-<lens>.txt" 2>&1 &
  pid=$!
  wait "$pid"
  echo "exit $?"
  ```

  `<id>` is the ticket id, `<n>` the checkpoint's number, and `<lens>` the lens. A
  second round uses `<lens>-2` in all three names. `< /dev/null` closes stdin; without
  it the run waits forever. `-o` holds only the last message. `>|` is for a shell with
  `noclobber`. Under `-s read-only` Codex cannot write files, so the brief tells it to
  put its complete findings in its final message, and the runner copies
  `last-<lens>.md` to `findings-<lens>.md` under a first line `Source: last-message`.
  If §14 on the ticket's base already prints a command that redirects stdin from
  `/dev/null`, use §14's, with Sol's model.
- **Completion.** A lens is complete when its process, followed by the process id the
  command captured, has exited 0 and `last-<lens>.md` is not empty. Never wait on a
  file name or a `pgrep` pattern, which can match a run in another worktree. A lens
  that exits non-zero or leaves an empty file runs again, once; a second failure stops
  the ticket.
- **Lenses.** `conformance` and `breakage`, as §14 defines them. A checkpoint marked
  "with a design lens" adds a third reviewer, `design`: does the model say what the
  Allium says, is anything the Allium allows missing without being listed under "Not
  yet modelled", and could a property hold for a reason other than the one claimed.
- **Brief.** It gives the ticket's path, `tickets/CONVENTIONS.md`,
  `docs/specs/pachinko.allium`, the checkpoint's questions, and the artefact. At a plan
  checkpoint the artefact is the plan file. At a checkpoint on a model or a recipe it
  is the files the ticket names there, with the output of the controls run so far. At
  a diff checkpoint it is `git diff "$base"...HEAD` plus the uncommitted changes.
  `base` is the commit step 1 records. The brief never gives the author's reasoning or
  the hand-back's conclusions.
- **Triage and apply.** Separate fresh subagents check each finding by a command or a
  read and write `ai_tmp/review/<id>-<n>/verdicts.md`. They apply valid fixes one
  writer at a time. Before the lead runs `just check` or `just verify-quint`, it
  confirms every writer has stopped.
- **After the last checkpoint.** A merge or a rebase onto the series base, or a fix made
  after the last checkpoint, as after a red CI run, that changes a file under the
  ticket's Files touched was not reviewed. The agent shows that diff
  at the push stop and asks whether the last checkpoint runs over it first.
- **Network.** Each checkpoint authorises its model call (§11, "Network").

`<id>` is `QM10`. Checkpoints:

- Checkpoint 1, in step 3: the plan and the fixture format, with a design lens, before
  any tracked file changes and before the **Stop** for the ruling on "Design proposed".
- Checkpoint 2, in step 9: the diff, before the final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | `replayInit` and `replayStep`, beside `init` and `step`; nothing else. Created by QM02 |
| `quint/mutants/*.patch` | shared, own lines | only a patch the model change breaks, re-cut with its mutation and its property kept (shared rule 11) |
| `quint/traces/pachinko_walk.json` | new | the fixture, written by `just quint-traces` and never by hand |
| `quint/traces/manifest.txt` | new | the manifest, written by the same command, with a comment naming decision 0022 and `just quint-traces` |
| `scripts/quint-traces.py` | new | the generator, the normaliser and the three modes |
| `Justfile` | shared, own lines | the `quint-traces` recipe with its comment; the comment over `check-quint` |
| `scripts/check-quint.sh` | shared, own lines | the hash check as a step of its own after the typecheck, and the header comment. Created by QM02 |
| `scripts/verify-quint.sh` | shared, own lines | the `just quint-traces diff` step and its comment. Created by QM03 |
| `crates/libpachinko/tests/queue/walk/replay.rs` | new | the replay test, the step mapping, the lease table and the list of what the replay does not compare |
| `crates/libpachinko/tests/queue/walk/model/trace.rs` | new | a `Model` built from a decoded state |
| `crates/libpachinko/tests/queue/walk.rs` | shared, own lines | the prologue as a function, `run`'s return value, the payload's `format!` as a function, `mod replay;` |
| `crates/libpachinko/tests/queue/walk/model.rs` | shared, own lines | `mod trace;`, and visibility only where `trace.rs` or `replay.rs` needs it |
| `crates/libpachinko/Cargo.toml` | shared, own lines | `serde_json` under `[dev-dependencies]` |
| `Cargo.toml` | shared, own lines | the decision-0007 note above `serde_json` |
| `Cargo.lock` | shared, own lines | libpachinko's entry, regenerated by `just lock`, never edited by hand |
| `docs/decisions/0007-dependency-policy.md` | shared, own lines | "Harness dependencies": `serde_json` for libpachinko's replay test |
| `docs/decisions/0022-quint-model-of-the-queue.md` | shared, own lines | conformance: what the replay compares, what it does not, the fixture's format, and the `cargo package` limit. Created by QM02; if it took another number, that file |
| `docs/how-to/work-with-the-specs.md` | shared, own lines | "Change behaviour", where QM02 says the model moves in the same change: that change also runs `just quint-traces` |
| `docs/reference/testing.md` | shared, own lines | "Framework": `serde_json`; "Tests from the specifications": the replay, beside the invariant walk |
| `docs/reference/commands.md` | shared, own lines | a `just quint-traces` row; the `check-quint` and `verify-quint` rows; the list of recipes outside `just check` |
| `docs/reference/quality-gates.md` | shared, own lines | what `check-quint` now checks, and that it stays offline |
| `docs/project/repository-map.md` | shared, own lines | `quint/traces/` and the script |
| `AGENTS.md` | shared, own lines | the same-change rule QM02 wrote names `just quint-traces`; the project map's `quint/` line names `quint/traces/` |
| `.agents/skills/spec-change/SKILL.md` | shared, own lines | the same sentence, where QM02 put the same-change rule |
| `CHANGELOG.md` | shared, own lines | libpachinko, unreleased, "Added": the replay test |
| `.scratch/quint-model/QM10-rust-replay.md` | ticket | `status:` and the hand-back notes |
| `.scratch/quint-model/README.md` | shared, own lines | QM10's Status cell; and, only if the maintainer's ruling on "Design proposed" differs from a recommended answer, the sentences it changes there |
| `.scratch/quint-model/QM08-*.md`, `QM09-*.md`, `QM11-*.md`, `QM12-*.md`, `QM13-*.md` | shared, own lines | only if that ruling differs from a recommended answer: the sentences it changes, each listed in the hand-back |
| `tickets/T39-decide-q2-q6.md`, `tickets/T40-pocket-order-model.md`, `tickets/T42-clock-model.md`, `tickets/T43-caller-contract.md`, `tickets/T45-drop-joining.md`, `tickets/T46-no-claim-without-records.md`, `tickets/T47-start-rule-and-config-bounds.md` | shared, own lines | only if that ruling differs from a recommended answer, and only in a ticket still `todo`: the paths and commands that its "The Quint model" paragraph and its `quint/pachinko.qnt` row take from this ticket, each listed in the hand-back |

QM08, QM09 and the T tickets list `quint/traces/`, the generator and `replay.rs`
conditionally, for the case that the tree they land on holds `quint/traces/`, at their
base or after their closing merge or rebase. If one of them landed first, its
conditional rows did not apply, and this ticket creates the files against the model as
it then stands.

The counts in this ticket are today's: 25 rules that a replay can reach, and a start
that has taken three joins. A T ticket that lands first changes the model and not this
text (README, "The T tickets that touch the model"), so step 2 counts the variants of
`Rule` in the base's model and names the T ticket behind any difference. If
`tickets/T45-drop-joining.md` landed first, the model has no join, and four things
in this ticket read differently. `replayInit` is not added: the generator passes
`--init=init`, and the manifest's `init` says so. `replayStep` offers six actions, not
eight. The witness list is the 23 rules that are left, and every "25" here reads 23.
The walk has no three joins for the replay to share, so `walk.rs` gives up only `run`'s
return value and the payload's function. Step 2's plan states which case the base is
in, with the lines of the model that show it, and the **Stop** in step 3 confirms it.

## Steps

1. **Worktree and baseline.** Confirm QM07 has merged into the series base the maintainer
   named (README, "Before QM01 starts"). Create the worktree on
   `ticket/qm10-rust-replay` from that base and record the base's full commit id as
   `base`. Check that `git ls-files .scratch/quint-model/` lists the README and this
   ticket. Run `just initialize` (network, authorised). Run `just check` on the untouched
   worktree and quote its last line, headed by the commit. Then run `just verify-quint`
   once, after the port probe of Verification shows nothing listening on
   `localhost:8822`, and quote its last lines: it is the baseline the model change is
   measured against.
2. **Read and probe, in scratch only.** Read everything Context names. Note each
   statement there that is no longer true at `base`, and how QM02 identified a lease and
   wrote `answer`. Count the variants of `Rule` in the model at `base`. This text
   assumes 25: QM02's three and QM04 to QM07's twenty-two. QM08 in the base adds five
   and QM09 two more, none of them replayed here. Trace any other difference to the T
   ticket behind it and name it in the plan: `tickets/T45-drop-joining.md` removes two
   rules and the joins that open every trace, and the paragraph after Files touched
   says what then reads differently. Copy the model to `ai_tmp/qm10/pachinko.qnt` and
   add `replayInit` and
   `replayStep` to the copy. On a base where T45 has landed there is no `replayInit`:
   add `replayStep` alone, and read `init` wherever this step, step 3's third
   question, step 4, claim C and the first acceptance criterion say `replayInit`.
   Then, quoting each command, its output and its exit status:
   - claim C: one `just quint run` over the copy with `--init=replayInit` and
     `--step=replayStep`; show state 0;
   - claim 14: two runs at seed 1 into two files; `cmp` fails on the raw files; a draft
     of the normaliser, kept in `ai_tmp/qm10/`, makes them equal; a run at seed 2 stays
     different once normalised;
   - claim 15: the `--mbt` run beside `last`;
   - claim B: `--observation=witness_PullWithinLease`, twice; then an observation that
     cannot be reached, `--observation=false`, which must exit non-zero and write no
     file;
   - the whole set at the recommended numbers into `ai_tmp/qm10/sample/`: the time it
     took, the size of the set in the recommended format and as raw ITF (claim E), the
     length of each witness trace, and any rule not reached;
   - claim A: write a throwaway test file, `crates/libpachinko/tests/qm10_probe.rs`,
     with one test that calls `std::fs::read_to_string`. Cargo builds a file directly
     under `tests/` as a test target of its own, so no tracked file is edited. Show
     `just effects` fail and quote the finding; replace the call with `include_str!`
     of `Cargo.toml` through `env!` and show `just effects` pass; delete the file and
     show `git status --short` prints nothing. It is the one file this step writes
     outside `ai_tmp/`; it is never tracked and is gone before checkpoint 1.

   Write `ai_tmp/qm10/plan.md`: `replayInit` and `replayStep` beside `init` and `step`;
   the format with a real trace from the sample; the manifest; the mapping table and the
   list of what the replay does not compare, checked field by field against `Model` and
   key by key against the model's state at `base`; the change to `run`; each control of
   steps 4 to 7; and each
   page's edit. No tracked file has changed.
3. **Adversarial review** (checkpoint 1, `tickets/CONVENTIONS.md` §14: conformance,
   breakage and design, over `ai_tmp/qm10/plan.md` and `ai_tmp/qm10/sample/`, before any
   tracked file changes). Questions:
   - Can the replay pass while the Rust and the model disagree: through a field the
     list fills with a constant, through the lease table, or through a step that both
     sides refuse for different reasons?
   - Does the normalised form drop anything a replay needs, or keep anything that
     differs between two runs, two machines or two checkouts?
   - Is `replayInit` exactly the state three joins reach, and is `replayStep` exactly
     `step` less the actions without a `WalkStep`, with the same arguments?
   - Does the set fire each of the 25 rules, and would the generation fail, rather than
     write a smaller set, if one could not be reached?
   - Can `just quint-traces check` pass on a fixture that was edited, emptied or
     generated from another model?
   - Does anything in the plan decide what the Rust should do from the Rust itself,
     rather than from the model (shared rule 1)?

   **Stop** for the maintainer's ruling on "Design proposed in this ticket", with the
   findings, the measured sizes and the times in hand. Record the ruling. If a rule
   could not be reached, or the set does not fit under 768 KB, say so here and ask; do
   not shorten the list of rules.
4. **The model.** Add `replayInit` and `replayStep` to `quint/pachinko.qnt`, or
   `replayStep` alone on a base where T45 has landed. Run
   `just check-quint`. Then, before any `verify`, show that each patch under
   `quint/mutants/` still applies to the changed model, in the way
   `scripts/verify-quint.sh` applies one, and re-cut each that does not, keeping its
   mutation and the property it names; list each in the hand-back (shared rule 11).
   The recipe checks every patch before its first run, so one broken hunk fails it
   before it can show anything else. Show nothing listens on `localhost:8822`, then run
   `just verify-quint`: `Canary` is violated on every instance, every property passes,
   every witness is reached and every mutant is caught by its property, as at the
   baseline. Never pass `--server-endpoint`, and run no `verify`, and no
   `just quint-traces`, which runs the evaluator, while another worktree verifies or
   while `just install-apalache` or `just install-quint-evaluator` runs (shared rule
   10).
5. **The generator, red then green.** Add `scripts/quint-traces.py` and the
   `quint-traces` recipe. Run `just quint-traces` and commit nothing yet. Quoting each:
   - `just quint-traces check` passes; `just quint-traces diff` passes and
     `git status --short` lists only this ticket's files (the diff wrote nothing);
   - a second `just quint-traces` leaves `git diff --stat -- quint/traces` empty against
     the first run's files (save them under `ai_tmp/qm10/` and `cmp`);
   - with one space appended to `quint/pachinko.qnt`, `just quint-traces check` fails
     and names `just quint-traces`; restore the model from a copy saved before the edit
     and show `cmp` exits 0 against it. The model holds step 4's uncommitted change, so
     `git diff --exit-code` cannot show the restore;
   - with `traces=0` in the manifest, then with the fixture's `traces` array emptied,
     then with one rank changed in one state of the fixture, `check` fails each time;
     with the `traces` array emptied and `fixture_sha256` updated to match, `check`
     still fails, on the count of traces; with one walk's `name` changed to another
     walk's and the hash updated, `check` fails, on the name given twice; with one
     rank changed and `fixture_sha256`
     updated to match, `check` passes and `diff` fails, which is why both exist;
     restore after each and show `cmp` exits 0;
   - with one rule added to the generator's list that no trace can reach, `write` exits
     non-zero and leaves the committed fixture and manifest untouched;
   - under §11's denial of the external network, in the form QM01's ticket uses for
     `just quint run`, `just quint-traces diff` exits 0, and both of §11's `curl`
     commands fail inside the same denial; quote all three exit codes.

   Then add the check to `scripts/check-quint.sh` and the step to
   `scripts/verify-quint.sh`. Show
   `just check-quint` fail on the appended space and pass once restored. Time
   `just quint-traces diff` and the whole of `just verify-quint` (claim 12).
6. **The dependency.** Add `serde_json` to libpachinko's development dependencies and
   extend the note in `Cargo.toml`. Run `just lock` (network, authorised) and show that
   `Cargo.lock` is the only lockfile `git status --short` lists and that its diff adds
   `serde_json` to libpachinko's entry and nothing else; if any version moved, in any
   lockfile, restore the lockfiles and **stop**. Run `just lock-check`. The other gates
   of claim H run in step 8, once a test uses the crate: until then `just deps-unused`
   fails, and that failure is the red control for it. Quote it.
7. **The test, red then green.** Write `walk/model/trace.rs`, `walk/replay.rs` and the
   changes to `walk.rs` and `walk/model.rs`. The suite has two tests:
   `the_queue_replays_every_model_trace`, and
   `the_fixtures_fire_every_replayable_rule`. The second lists the 25 rules and fails
   unless three things hold: for each of them the set has a trace named
   `witness_<Rule>` whose last state has that rule in `fired`; the manifest's
   `witnesses` is the same list; and the union of `fired` over the set is exactly the
   25. The union alone would not notice a missing witness, since a rule such as
   `HoldRecord` fires on the way to many others. Run `just test` and show
   both pass, with the number of traces and steps replayed printed by the first. Then,
   each on an edit to the fixture restored from a saved copy afterwards, show
   `just test` fail and quote the assertion, which begins with the prefix "A mismatch
   names its comparison" gives it and names the comparison in brackets below:
   - one record's rank changed in one state (`state`);
   - two records of one batch swapped in an `answer` (`answer`);
   - the lease argument of one `ack` in `last` changed to another lease of the same
     trace (`lease`, or `refused` or `state` if the table is right and the step then
     lands elsewhere; say which, and why);
   - a refused step's `fired` given a rule (`refused`);
   - the `traces` array emptied, and `traces=0` in the manifest (`count`);
   - the witness trace of one rule that other traces also fire removed, with the
     manifest's count lowered to match (`replay rules:`, naming the missing witness);
   - a key the replay does not know added to one pocket of one state, and one `last`
     given a variant the mapping does not know (`list`).

   None of these runs `cargo clean` (claim F). After the last restore, `cmp` exits 0
   for the fixture and for the manifest against the copies saved before the first
   edit. Neither file is committed yet, so `git diff` says nothing about them. If the
   unedited replay fails, the
   cause is one of three, and shared rule 2 says which to fix: the mapping is wrong, fix
   the test; the model transcribes the Allium wrongly, which is a model change with its
   own review, so **stop** and ask; or the Rust departs from the Allium, which is a
   finding for the maintainer, so **stop**, quote the trace and the step, and change
   neither.
8. **Gates and words.** Run `just fmt-check`, `just clippy`, `just metrics`,
   `just effects`, `just features`, `just deny`, `just deps-unused`, `just coverage`,
   `just doc` and `just test-doc-default`, and fix each failure at its root with the
   `fix-quality` skill. Edit each page in Files touched. Run `just check-docs` and
   `just check-agents`.
9. **Adversarial review** (checkpoint 2, `tickets/CONVENTIONS.md` §14: conformance and
   breakage, over `git diff "$base"...HEAD` plus the uncommitted changes, before the
   final commit). Questions:
   - Could `just test` pass with the replay compiled out, skipped, or reading a fixture
     other than the committed one?
   - Could `just check-quint` or `just verify-quint` pass after a model edit that
     changes a trace, and does `check-quint` stay offline and read-only?
   - Does the change to `run` alter what the proptest walk asserts, or the cases it
     draws?
   - Is every field of `Model`, and every key of a fixture's state, either compared or
     in the list, does each row name the ticket that will map it, and does the test
     fail on a key or a step that is neither?
   - Do `Cargo.lock`, the dependency note and decision 0007 agree, and did any version
     move?
   - Does every page in Files touched say that the replay samples, and what it does not
     compare?
10. **Commit and hand back.** Fetch, and merge or rebase onto the series base (shared
    rule 13). If that changed `quint/pachinko.qnt`, because QM08, QM09 or a T ticket
    landed meanwhile, every condition in this ticket that turns on what the model
    holds is read again against the merged tree: run `just quint-traces` before any
    gate, edit the list as "Who edits the list" says, recount the rules as step 2
    does, rerun step 7's green run and step 8's gates, and show the merge's diff at the
    push stop (the review deltas, "After the last checkpoint"). Commit the work. Rerun
    Verification and quote it, headed
    by the full commit id. Record each claim's outcome and each mutant patch re-cut. Set
    `status: done` here and in the index of `.scratch/quint-model/README.md`, write the
    hand-back notes, and commit the record. Then run `just check` once more, on the
    record's commit, and show its last line and an empty `git status --short`: the
    quoted Verification ran on the commit before, and the hook gate has not yet read
    the notes. If it fails, fix the notes, commit and run it again; the work's commit
    is not amended. **Stop** before pushing or opening a pull request: both need
    authorisation (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").
    Claim D can be shown only after the push.

## Acceptance criteria

- `quint/pachinko.qnt` has `replayInit` and `replayStep` and no other change, or on a
  base where T45 has landed `replayStep` alone, with the manifest's `init=init`, shown by
  `git diff "$base"...HEAD -- quint/pachinko.qnt`; `replayStep` offers no `lapse`,
  `advance` or `tick`.
- `just verify-quint` passes as shared rule 6 asks, with `Canary` violated on every
  instance, and now also runs `just quint-traces diff`; its output is quoted, after the
  port probe.
- `just quint-traces` twice writes the same two files, byte for byte, and the fixture is
  under 768 KB; both are quoted.
- `just quint-traces check` fails on a one-character model edit, on `traces=0`, on an
  emptied fixture, with its hash left and with its hash updated, and on an edited
  fixture; `just quint-traces diff` fails on a fixture edited with its hash updated;
  each is quoted with its exit status.
- `just check-quint` fails on the one-character model edit and passes once it is
  restored, with no network.
- `just test` runs `the_queue_replays_every_model_trace` and
  `the_fixtures_fire_every_replayable_rule`, and each of step 7's seven edits makes it
  fail on the assertion named there, shown by the message's prefix.
- The fixture holds, for each of the 25 rules QM02 and QM04 to QM07 own, a trace named
  for the rule that ends with it fired, and no state's `last` is a lapse, an advance or
  a tick.
- The record's commit passes `just check` too, run after it was made.
- `replay.rs` holds one list of what the replay does not compare. Every field of `Model`
  and every key of a fixture's state is either compared or in it, each row names the
  ticket that will map it, and a key or a step that is neither fails the test.
- Every trace in the fixture comes from `pachinko_walk`; the manifest's `main` says so.
- `serde_json` is a development dependency of libpachinko only; `Cargo.lock` moved no
  version; the note in `Cargo.toml` and decision 0007 name the test.
- `just effects`, `just metrics`, `just deps-unused`, `just deny`, `just lock-check` and
  `just coverage` pass.
- Each claim of "Claims this ticket checks" has its outcome recorded; claim D is
  recorded as not shown until the CI job has run.
- Both checkpoints are recorded as §14's Record asks, with the deltas above, and the
  maintainer's ruling at step 3 is quoted with its date.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run from a file as one `sh` script, after the commit; `set -eu` stops it at the first
command that fails.

```sh
set -eu
git rev-parse HEAD
mkdir -p ai_tmp/qm10

# Whatever stops the script, the file being edited is put back from its saved copy.
saved=""
target=""
restore() {
  if test -n "$saved"; then cp "$saved" "$target"; fi
}
trap restore EXIT
trap 'exit 1' HUP INT TERM

# The hash check: green, red on one appended space, green again.
just quint-traces check
just check-quint
cp quint/pachinko.qnt ai_tmp/qm10/model.saved
saved=ai_tmp/qm10/model.saved
target=quint/pachinko.qnt
{ cat "$saved"; printf ' '; } >| "$target"
if cmp -s "$saved" "$target"; then exit 1; fi
if just check-quint >| ai_tmp/qm10/red-model.log 2>&1; then exit 1; fi
cp "$saved" "$target"
saved=""
grep -F 'just quint-traces' ai_tmp/qm10/red-model.log
git diff --exit-code quint/pachinko.qnt
just check-quint

# Zero traces fail the hash check and the test.
cp quint/traces/manifest.txt ai_tmp/qm10/manifest.saved
saved=ai_tmp/qm10/manifest.saved
target=quint/traces/manifest.txt
sed 's/^traces=.*/traces=0/' "$saved" >| "$target"
if cmp -s "$saved" "$target"; then exit 1; fi
if just quint-traces check; then exit 1; fi
if just test >| ai_tmp/qm10/red-zero.log 2>&1; then exit 1; fi
cp "$saved" "$target"
saved=""
grep -E 'replay .*: count:' ai_tmp/qm10/red-zero.log

# A hand-edited fixture fails the test: the first state's next_position becomes 99.
cp quint/traces/pachinko_walk.json ai_tmp/qm10/fixture.saved
saved=ai_tmp/qm10/fixture.saved
target=quint/traces/pachinko_walk.json
awk '!done && sub(/"next_position":[0-9]+/, "\"next_position\":99") { done = 1 } { print }' "$saved" >| "$target"
if cmp -s "$saved" "$target"; then exit 1; fi
if just test >| ai_tmp/qm10/red-fixture.log 2>&1; then exit 1; fi
cp "$saved" "$target"
saved=""
grep -E 'replay .* step [0-9]+: (start|state):' ai_tmp/qm10/red-fixture.log
git diff --exit-code -- quint/traces
just test

# The fixture's size, under the hook's 768 KB.
size=$(wc -c < quint/traces/pachinko_walk.json | tr -d ' ')
test "$size" -gt 0
test "$size" -lt 786432

# No lapse, advance or tick in a fixture. The control finds a step that is there.
grep -c -F '"Pull"' quint/traces/pachinko_walk.json
if grep -E '"(Lapse|Advance|Tick)"' quint/traces/pachinko_walk.json; then exit 1; else test "$?" -eq 1; fi

# Regeneration: the same bytes, then the whole verify recipe on the pinned checker.
just quint-traces diff
python -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(5)' &
listener=$!
sleep 1
lsof -nP -iTCP:8822 -sTCP:LISTEN
kill "$listener"
wait "$listener" || true
sleep 1
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi
just verify-quint

just effects
just deps-unused
just lock-check
just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

The four `if just ...; then exit 1; fi` lines after an edit are the red tests, and
the plain `just check-quint`, `just quint-traces check`, `just test` and
`just quint-traces diff` lines are the green ones: a check that failed on the committed
files too would stop the script there. Each `cmp -s` line is the positive control for
the edit before it, since an edit that changed nothing would make the red test pass for
the wrong reason. The `grep` after each red run shows it failed in the hash check or in
a comparison of the replay, by the message's prefix ("Design proposed", "A mismatch
names its comparison"), and not somewhere else: the test's name alone would also match
the line that reports it passing while another test failed. The fixture edit changes
a field that only a state holds and that the replay compares on its own, the queue's
next position, to a value no trace of this length reaches, so it cannot land on an
argument of a refused step and leave the replay green; the first one in the file is
in a trace's first state, so the comparison is `start`, or `state` if the test words
it so. Step 7 holds the edits aimed at the other comparisons. While `saved` is set,
the `trap` puts the edited file back however the script ends, an interrupt included.
`"Pull"`, `"Lapse"`, `"Advance"` and `"Tick"` are
the variants of `Step` as QM02 spells `Pull`, and `"next_position":` is the counter as
the fixture is expected to spell it; if the model spells either otherwise,
adjust the pattern and say so under Departures. `grep -c -F '"Pull"'` is the positive
control for the silent probe after it,
and the first `lsof`, run while the control listener holds the port, is the positive
control for the two that must print nothing. Step 5 holds the other mutations and the
network denial, and step 7 the other six edits; their output is quoted there.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1

### Checkpoint 2

### Departures

### Verification

## Open points

- **Claim D, the same traces on both platforms.** Nobody can show it before a push. If
  the CI job `quint` regenerates different bytes on `linux-64`, the fixtures cannot be
  committed from a Mac as designed. The maintainer settles what then: generate in CI
  only, or record a fixture per platform. Raised at the first pull request.
- **The numbers.** 25 witness traces of at most 12 steps and four walks of 30 are
  starting values nobody has measured. The maintainer rules at step 3, with step 2's
  sizes and times.
- **A rule the search does not reach.** If `--max-samples=20000` at 12 steps does not
  reach one of the 25 rules, the choices are more samples, more steps or a second seed
  for that rule. The maintainer rules at step 3; the rule is never dropped from the list.
- **`cargo package`.** A packaged libpachinko could not build this test. Both crates are
  `publish = false`. Whoever first decides to publish libpachinko settles where the
  fixture then lives.
- **T43 and T45.** T43's tickets replace two rows of the list (the refusal's name and
  the lease id's value) and change what each action answers. T45 removes `replayInit` and the
  three joins. Each regenerates the fixtures in the same change. Settled by those
  tickets when they land.
- **QM08, and QM09 after it, beside this ticket.** QM08 and QM10 are the README's
  parallel pair, and QM09 may land on either side of this ticket too. Whichever of
  QM10 and one of them lands second runs `just quint-traces`, because the model's
  bytes changed, edits the list as "Design proposed" says, and reruns its
  verification. The condition is what the tree holds after the closing merge or
  rebase, not what the base held. Settled at each second merge.
- **`pachinko_zero_ttl`.** QM09 adds it. This ticket takes no fixture from it. Whether
  it should ever have one is QM11's point, and waits for T42's ruling on what one tick
  does there.
- **The expiry an answer carries.** The Rust's batch and its renew return the lease's
  `expires_at`; the model's `answer` holds none. This ticket compares the returned
  value with the lease's stored one. Whether `answer` should carry it is the
  maintainer's, at step 3, and a change to QM02's ghost if so.
- **`fired` against the Rust.** The replay compares the Rust's success value with the
  rule fired for a publish and a release only. Whether the library should report more is
  not this series' question; QM12 compares `fired` with the double, which does report it.
