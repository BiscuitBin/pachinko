---
id: QM11
title: "Conformance: lapses and time"
status: todo
depends_on: [QM09, QM10]
parallel_with: [QM12, QM13]
branch: ticket/qm11-replay-lapses-and-time
estimated_size: S
---

# QM11: Conformance: lapses and time

## Context

Two tickets come before this one. QM09 gave the model in `quint/pachinko.qnt` the clock:
`now`, every timestamp, the three TTLs, both decay rules, and the actions
`advance(millis)` and `tick` in `step`. It also added two model-only actions to `step`,
`decaySuperseded(entry)` and `decayDeadLetter(entry)`, which decay one entry as
`lapse(lease)` lapses one lease, and a second instance, `pachinko_zero_ttl`. It added
nothing to `replayStep`. QM10 tied the model to the Rust: it added `replayStep`, the step
relation the
trace fixtures are generated from, committed the fixtures under `quint/traces/`, and wrote
a test in libpachinko's queue suite that replays them. `replayStep` offers
`publisherJoins`, `consumerJoins`, `publish`, `pull`, `ack`, `nack`, `renew` and `release`
and nothing else. So no fixture moves the clock, no fixture lapses a lease, and QM10's
test does not read a timestamp from a fixture: it expects the constant the Rust must
stamp while the clock stands still.

This ticket extends the fixtures and the mapping to `advance` and `tick`, and reads every
timestamp from the fixture. It owns no rule, invariant or guarantee.

What QM10 built, to be read at the ticket's base:

- `replayInit` and `replayStep` in `quint/pachinko.qnt`. `replayInit` is the state after
  the walk's three joins.
- `scripts/quint-traces.py`, run by `just quint-traces`. Mode `write` generates every
  trace with `just quint run` at a recorded seed, normalises it, and writes
  `quint/traces/pachinko_walk.json` and `quint/traces/manifest.txt`. Mode `check`, which
  `just check-quint` runs, compares the manifest's two hashes with the model and the
  fixture, offline. Mode `diff`, which `just verify-quint` runs, generates again and
  compares byte for byte. The set is one witness trace for each rule a replay can reach,
  found with `--observation=witness_<Rule>`, and a few plain walks.
- `crates/libpachinko/tests/queue/walk/replay.rs` and
  `crates/libpachinko/tests/queue/walk/model/trace.rs`. For each trace the test runs the
  walk's three joins, then maps each state's ghost `last` to a `WalkStep`, applies it
  through the walk's `run` in `crates/libpachinko/tests/queue/walk.rs`, and compares:
  whether the step was accepted, the ghost `answer` with what the queue answered, the
  state through `Model::of`, and the model's ended leases with `Walk::ended`. A lease is
  named through a table from the model's lease to the `LeaseId` the Rust returned.
- One list in `replay.rs` of what the replay does not compare, which shared rule 11
  keeps complete: a key of a fixture's state, or a step in a fixture, that is neither
  compared nor listed fails the test. Its timestamp rows name this ticket. If QM09
  merged after QM10, QM09 added rows for `now` and for each timestamp the fixture now
  holds, naming this ticket too. This ticket maps every row that names it.

If any of that differs at the base, because the maintainer ruled otherwise at QM10's
step 3, read QM10's hand-back and follow what landed.

What the Rust does, read at commit `cf1f3f3`, not run:

- **A tick is one pass at one `now`.** `tick_counting` in
  `crates/libpachinko/src/queue/expiry.rs` (lines 127 to 142) reads `now` once (line
  128), takes the due leases (line 129), lapses each in that order (lines 130 to 134),
  and then decays (line 135). `TickReport` (lines 47 to 54) carries the lapsed leases in
  that order and two counts.
- **The order of lapses is earliest `expires_at` first, then lease id.** `due` in
  `crates/libpachinko/src/queue/leases.rs` (lines 106 to 117) walks
  `by_expiry: BTreeSet<(Timestamp, LeaseId)>` (line 37). The test's `CountingLeaseIds`
  counts up, so there a tie is broken in grant order. The walk asserts this order
  (`walk.rs` line 520), and its own model sorts the same way (`walk/model.rs` lines 566
  to 572).
- **The shed queues are kept by expiry, then arrival.** `Shed` in
  `crates/libpachinko/src/queue/shed.rs` keys each queue by `(Timestamp, u64)` (lines 46
  to 50), and `decay` (lines 70 to 78) takes the due prefix of the superseded records and
  then of the dead letters.
- **The queue's `now` never goes back.** `Queue::now` in `crates/libpachinko/src/queue.rs`
  (lines 276 to 279) is `self.latest.max(self.clock.now())`, and `latest` (lines 118 to
  120) is "The latest instant the queue has read". The unit test
  `now_never_goes_back_when_the_clock_does` (from line 307) pins it.
- **The queue reads its clock lazily.** The comment on `Walk::now` (`walk.rs` lines 168
  to 173) says the queue reads its clock in an accepted step and in a publish by a joined
  publisher, "never in a refused lease action, a `NoPocketAvailable` pull, a join or an
  `Advance`".
- **The walk's two clock steps.** `WalkStep::Advance(i64)` (line 114) moves the walk's
  clock and the fake clock, forwards by up to `FORWARD_MILLIS`, 120, and back by up to
  `BACKWARD_MILLIS`, 120 (lines 50, 54, 141 and 142). `WalkStep::Tick` (line 116) calls
  `Queue::tick`. The walk starts its clock at `START`, 1,000 ms (line 46).

What the Allium says, read at the same commit: `LeaseLapses` is
`when: lease: Lease.expires_at <= now` (`docs/specs/pachinko.allium` lines 726 to 730), and
the Lease expiry comment above it says the clock emits `LeaseLapsed` "once per active
lease whose expires_at has passed". Neither orders two leases due at the same `now`.
`tickets/T41-tick-order-investigation.md` exists because of that: its Context says the
module "gives no order among several that are due at one `now`", it investigates and rules
nothing, and `tickets/T42-clock-model.md` lands whatever is ruled. T41 also records the
ruling of 2026-11-24 that a tick drains: one `now`, every due lapse, then every due decay.
QM09 waited for both, so the Allium at this ticket's base may say more. Step 2 reads it.

The two mismatches this ticket bridges:

1. **One lease or one entry per step against the whole due set.** QM08's `lapse(lease)`
   lapses one lease, at any moment, and QM09's `decaySuperseded(entry)` and
   `decayDeadLetter(entry)` each decay one entry. All three stay in `step` alone. The
   Rust has no such call: `Tick` lapses every due lease and then decays everything due.
   So none of the three can be replayed, and only a model step that does a whole tick
   can map to `WalkStep::Tick`.
2. **A clock that can step back against a `now` that cannot.** A negative `Advance` moves
   the Rust's clock back, and the queue's `now` stays where it was, but only from the last
   instant the queue *read*. Start at 1,000 ms, advance by 100, advance by -50, publish:
   the queue never read 1,100, so it stamps 1,050. A model with one `now` that ignores a
   negative advance would stamp 1,100. A replay of such a step compares the wrong thing.

**One more blocker, with no id yet: the ticket that lands the ruled clock in the Rust.**
The README's "Dependencies" says "QM11 also needs the Rust to hold the ruled clock. If
the ticket that lands the clock in the Rust is not the specification ticket, QM11 waits
for it as well." T42 creates those tickets and none has an id. This ticket's
`depends_on` cannot name it. Whoever creates it adds its id to this ticket's
`depends_on` and to the README's index, and step 1 checks for it either way.

**One instance.** Every fixture comes from `pachinko_walk`. `pachinko_zero_ttl` is left
out of the replay until T42's ruling says what one tick does there: with a zero TTL a
lapse makes entries that are due at the same `now`, QM09's `tick` drains until nothing
is due, and the Rust's tick is one pass (`expiry.rs` lines 127 to 142). Whether those
two agree is what T41 investigates and T42 rules.

Read these:

- `.scratch/quint-model/README.md`, in full, and
  `.scratch/quint-model/QM09-clock-timestamps-decay.md` and
  `.scratch/quint-model/QM10-rust-replay.md` with their hand-back notes.
- `tickets/T41-tick-order-investigation.md` and the page it writes,
  `docs/explanation/tick-order.md` (created by T41); `tickets/T42-clock-model.md`, its
  hand-back, and the tickets it created.
- `docs/specs/pachinko.allium`: the comments above "Lease expiry", "Lease end" and
  "Decay", and whatever T42's specification ticket added about `now` and a tick.
- `crates/libpachinko/src/queue/expiry.rs`, `leases.rs` and `shed.rs` in the same
  directory, and `crates/libpachinko/src/queue.rs`.
- `crates/libpachinko/tests/queue/walk.rs` (`Advance`, `Tick`, `tick`, `advance`,
  `Walk`), `walk/model.rs` (`tick`, `lapse`), `walk/replay.rs` and `walk/model/trace.rs`.
- `scripts/quint-traces.py`, the `quint-traces`, `check-quint` and `verify-quint`
  recipes in `Justfile`, and `scripts/check-quint.sh` and `scripts/verify-quint.sh`
  (created by QM02 and QM03).
- `tickets/CONVENTIONS.md` §11 and §14.

This ticket changes Rust under `crates/`, so `AGENTS.md`'s block for that applies to the
agent that runs it: use the `rust-change` skill, and `review-change` to review the
result. Use `review-docs` for the pages and `plan-change` to pick the ticket up.

## Goal

`replayStep` offers `advance` and `tick` as well. The fixtures under `quint/traces/` hold
traces in which the clock moves, leases lapse, pockets park and revive, and shed entries
decay, with a witness trace for each of the seven rules that needed the clock. The Rust
test maps `advance` to `WalkStep::Advance` and a whole tick to `WalkStep::Tick`, reads
every timestamp from the fixture, and compares what a tick lapsed and decayed with the
model's. A fixture with a wrong expiry fails the test. The model's bare `lapse` steps
and its two single-entry decay steps stay checked for safety by `just verify-quint` and
are not replayed. Every fixture comes from `pachinko_walk`.

### Owns

| Kind | Names |
| --- | --- |
| Rules | none |
| Invariants | none |
| Guarantees | none |
| Partial or extended, not counted | Extends `replayStep`, which QM10 created, with `advance` and `tick`. It adds no rule, no property and no state |

## Design proposed in this ticket

The README's design point for QM11 is "Which `tick` steps and which `advance` steps can
be replayed". The first three bullets answer it; the rest are this ticket's own. The
maintainer confirms or changes them at the **Stop** in step 3, and the hand-back records
the ruling. QM09 keeps `tick` and `advance` in `step`; this ticket adds them to
`replayStep`.

- **Only a whole tick maps to `WalkStep::Tick`.** Recommended: `replayStep` gains the
  model's `tick`, provided the `tick` QM09 landed is one action that, at one `now`,
  lapses every active lease whose `expires_at` is at or before `now` and then removes
  every shed entry whose `expires_at` is at or before `now`, with no other step between.
  That is the Rust's `Tick` and the ruling that a tick drains, and it is what QM09's
  "Design proposed" says of `tick`: every due lapse, then every due decay, until nothing
  is due at that `now`. `lapse(lease)`, `decaySuperseded(entry)` and
  `decayDeadLetter(entry)` are never added to `replayStep`. If the `tick` that landed is
  anything else, this ticket does not write a
  second tick into the model: it **stops** and asks, because a new top-level action is a
  model design and not a mapping. Alternative, not recommended: generate with `lapse` in
  the step relation, keep only traces in which every due lease lapses before any other
  step, and fold each such run of `lapse` steps into one `Tick`. It needs a filter that
  knows what "due" means, in a model where an untimed `lapse` may fire for a lease that
  is not due, and it replays the decays not at all.
- **The order of lapses due at the same `now` is not this ticket's to choose.** Step 2
  reads what the Allium at the base says. There are three cases.
  1. It fixes earliest `expires_at` first and then the Rust's tie-break. The model's
     `tick` follows it. The replay compares `TickReport`'s `lapsed` in order only
     against an order the fixture records: two states show
     which leases lapsed, not in what order. QM09's ticket has `last` record the order
     of a tick's lapses in every case, fixed by the Allium or not; step 2 reads
     whether the model at the base does. Recommended: if it does, compare `lapsed`
     with that order, in order. If it does not, compare `lapsed` as a set, and
     **stop** at step 3 to ask
     whether QM09's `tick` should record its order in every case, which is a model
     change with its own review. The fixture then shows the order only where two
     lapses of one tick add entries to one shed queue, which the state comparison
     reads, and the Rust's own order stays asserted by the walk (`walk.rs` line 520),
     which the replay runs through. Never rebuild the order by sorting the leases that
     went from active to expired: that compares the Rust with the replay's own reading
     of the Allium, and passes whatever order the model took.
  2. It fixes another order. Then the Rust departs from the Allium. That is a finding
     for the maintainer and for the ticket that lands the clock in the Rust; this ticket
     **stops**.
  3. It fixes none. Then QM09's `tick` takes the due lapses in any order, chosen at the
     step and recorded in `last`, so a fixture holds one order and says which, and the
     Rust holds its own. The order shows in two places: the order of
     `TickReport`'s `lapsed`, and the order in which two lapses of one tick add entries
     to the same shed queue. Read, not run: it shows nowhere else, because each lapse
     touches its own lease, consumer and pocket, and none of the three expiry rules
     allots a position. This case goes to the maintainer as an Open point, with the
     options written there and none chosen.
- **Which `advance` steps are replayed depends on which clock QM09 landed.** QM09
  proposes one of three clocks, chosen by what the landed Allium does with a reading
  behind the latest time (QM09, "How the clock moves"), and this ticket follows
  whichever is in the model at the base.
  1. *One `now` that never decreases.* `advance` adds a non-negative amount, and a
     negative `advance` is a step no rule answers: it changes nothing. The Rust's
     `Advance` with a negative value does change something, its clock (Context,
     mismatch 2). Recommended: `replayStep` offers `advance(millis)` for `millis` from 0
     to the largest value `pachinko_walk` gives it, and no negative value. With a clock
     that only moves forwards, the instant the Rust stamps at any step is the model's
     `now`, whether or not the queue read its clock in between. The Rust's hold stays
     covered by `now_never_goes_back_when_the_clock_does` and by the proptest walk's
     backward advances.
  2. *Two values, and a reading behind the latest time is held: `clock`, moved by
     `advance` from -120 to 120, and `now`, the latest instant read, which a step that
     reads sets to the greater.* That is what the Rust
     does, so a negative advance can be replayed. Recommended: `replayStep` offers every
     `advance` the instance has, provided step 2 shows three things. QM09 recommends
     answers under which the second and the third hold, a `now` that starts below
     every reading and a clock that stops at 0; step 2 reads what landed.
     - *The same reads.* The model's steps that read the clock are exactly the Rust's
       (`walk.rs` lines 168 to 173): an accepted step and a publish by a joined
       publisher, never a refused lease action, a `NoPocketAvailable` pull, a join or
       an advance.
     - *The same first `now`.* Read, not run: the Rust's latest instant read is not
       1,000 before a step reads the clock. `walk` sets only `clock` to `START` and
       leaves `Walk::now` at `Timestamp`'s default (`walk.rs` lines 647 to 651), and no
       join reads the clock. From there `advance(-50)` and a publish stamp 950 in the
       Rust. A model whose `now` starts at 1,000 stamps 1,000.
     - *The same floor.* The walk's `advance` saturates (`walk.rs` line 543,
       `saturating_add_signed`), and nine advances of -120 from 1,000 fit in one
       witness trace. A model `clock` that is a plain integer goes to -80 where the
       Rust's stays at 0, and the next forward advance lands on another instant.

     If any of the three differs, replay forwards only and bring the difference to the
     **Stop**; it is a question about the Allium, the model or the Rust, not a mapping,
     and this ticket changes none of them. With forward advances only, none of the three
     can change a stamp: the clock never falls below the latest read or near 0. Forward
     only settles which advances a fixture holds and nothing more. A first `now` or a
     set of reading steps that differs still fails the comparison of `now` below, at
     state 0 or at the first step one side reads and the other does not, so step 4
     does not start until the maintainer has ruled on that comparison as well.
  3. *Two values, and a reading behind the latest time is refused.* The same `clock`
     and `now`, but a rule that reads the clock answers a step only when `clock` is at
     or after `now`; when the clock is behind, no rule answers and nothing changes. The
     Rust does not do that: it holds the latest time and accepts the step. So after an
     advance back the model refuses a step the Rust accepts, and a replay of it fails
     on accepted or refused. Recommended: `replayStep` offers no negative `advance`
     under this clock, as under one `now`. With forward advances only the clock is
     never behind `now`, the refusal never applies, and the two agree. The difference
     itself is not a mapping and not this ticket's to settle: it is a finding for the
     ticket that lands the ruled clock in the Rust (Context, "One more blocker"),
     written under Open points and in the hand-back with the Allium's lines and
     `fn now` in `crates/libpachinko/src/queue.rs` quoted. If that ticket has merged
     and the Rust refuses too, step 2 says so and the finding is closed.

  Under all three `advance(millis)` maps to `WalkStep::Advance(millis)`, and the model's
  clock starts at 1,000, the walk's `START`, so a model instant is the Rust's
  `Timestamp` with nothing added. The replay still converts in one function. What it
  compares after every step, state 0 included: with one `now`, the model's `now` with
  `Walk::clock`; with two values, under either such clock, the model's `clock` with
  `Walk::clock` and the model's `now` with `Walk::now`. A different start fails at
  state 0. Under the two
  values, a `now` that differs because one of the three things above differs fails
  too, as it should: what the replay compares in that case is part of the ruling at
  the **Stop**, and is never settled by leaving `now` out.
- **Every timestamp is read from the fixture.** This ticket maps every row of the list
  in `replay.rs` that names QM11, and removes it: a lease's `granted_at` and
  `expires_at`, an ended lease's `ended_at`, a batch's and a renew's `expires_at`, a
  shed entry's `shed_at` and `expires_at`, a held record's and a dead letter's
  `nacked_at`, a dead letter's `expires_at`, and `now` and whatever else QM09 listed
  for the clock. A batch's and a renew's `expires_at` are values the Rust returns, and
  the model's `answer` holds no expiry (README, second design block; QM10, "What is
  compared after every step"). Each is compared with that lease's `expires_at` in the
  fixture's state after the step, so a returned value that differs from the stored one
  fails. If `run` at the base does not hand the two values back, this ticket adds
  them. If a row for `consecutive_expiries` is still there and names no later
  ticket, this ticket maps it too. The rows that stay are `pocket_order`, a refusal's
  name and a `LeaseId`'s value, each with the T ticket that will map it. The list stays
  complete: a key or a step that is neither compared nor listed still fails the test.
- **What is compared for a tick.** `run` hands back the `TickReport` for a `Tick`, as it
  hands back a batch for a pull. The replay compares:
  1. the leases whose status went from active to expired between state `i - 1` and
     state `i` with `lapsed`: the same leases through the lease table, each with its
     pocket, consumer, `granted_at`, `expires_at` and `Expired` with `ended_at` equal to
     the tick's `now`. The order is compared by one rule, the lapse order's cases
     above: in the order `last` records where the Allium fixes the order and `last`
     records it (case 1); as a set where the Allium fixes it and `last` does not; and
     where the Allium fixes none (case 3), as the maintainer rules at step 3 among the
     Open point's options, and never in order before that ruling, since the model's
     order and the Rust's may then both be legal and differ;
  2. the number of entries that left each shed queue with `decayed_superseded` and
     `decayed_dead_letters`;
  3. the state after the tick through `Model::of`, shed queues entry by entry;
  4. the model's ended leases with `Walk::ended`, now with `released` or `expired` and
     `ended_at`.
- **Accepted and refused, for the two new steps.** QM10 compares "`fired` is empty" with
  "the Rust returned an error". Neither new step can be refused. For `advance`, `fired`
  is empty and nothing but the clock changes. For `tick`, `fired` is empty exactly when
  `TickReport::is_empty()`.
- **Seven more witness traces.** The generator's list of rules grows from 25 to all 32:
  `LeaseLapses`, `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront`,
  `LeaseExpiryIdlesEmptyPocket`, `ReviveParkedPocket`, `SupersededRecordDecays` and
  `DeadLetterDecays` are reachable once `replayStep` has `advance` and `tick`. The
  shortest path to `DeadLetterDecays` is a publish, a pull, a nack, a release, three
  advances (the walk's `dead_letter_ttl` is 250 ms and one advance is at most 120) and a
  tick. Every run still passes `--main=pachinko_walk`. If the search at QM10's
  `witness_steps` and `witness_samples` does not reach a
  rule, raise them and say so at the **Stop**; never drop the rule. The plain walks keep
  QM10's count and length. The fixture stays one file under the hook's 768 KB
  (`.pre-commit-config.yaml` lines 92 and 93); a set that does not fit is a **Stop**.
- **One more trace, for a tick that lapses two leases.** A witness per rule does not
  give one: each of the seven can be reached by a tick that lapses a single lease, and
  no plain walk has to hold the case. Recommended: one more run of the same shape as a
  witness run, whose `--observation` is an expression in place of a definition's name
  (`cli.js`: "a definition name or an expression"; read, not run, claim I). The
  expression holds when `last` is a tick and at least two leases that are active in
  `prev` are expired in the state. It is written in `scripts/quint-traces.py` against
  the model's names and adds nothing to the model. The trace's `name` is
  `tick_lapses_two`; the manifest gains the key `cases`, which lists such names, and
  `traces` equals `walks` plus the numbers of `witnesses` and `cases`, which `check`
  enforces, with each name in `cases` held by exactly one trace, as it requires of a
  witness and a walk. `write` fails, and writes nothing, if the run does not reach it.
  The rule
  test requires a trace of that name whose last step is a tick that took two leases
  from active to expired. The shortest path is two publishes into two pockets, two
  pulls by two consumers, one advance past `lease_ttl` and a tick. A tick that decays
  an entry needs no such run: in `replayStep` only a tick fires a decay rule, so each
  decay rule's witness trace ends with one. Alternative: a `val` in the model for the
  observation. It is a model definition outside `replayStep`, and QM03's helper refuses
  a `witness_` name whose suffix is not a rule. Not recommended.
- **The shed queues' two orders agree, and the replay shows it.** The model keeps each
  shed queue as a list in creation order. The Rust keeps it by `expires_at` and then
  arrival. With one TTL for each queue and a `now` that never decreases, an entry
  created later never expires earlier, so the two orders are the same. This is an
  argument, not a run. The state comparison in every step is what checks it; if it
  fails there, the cause goes through shared rule 2 and is not patched in the mapping.

## Claims this ticket checks

No row of the README's "Claims the tickets check" names QM11 alone. Row 12 names every
ticket that adds a run, and this one adds eight: seven witness runs and the run for a
tick that lapses two leases. The lettered rows are claims this ticket rests on that
nobody has run.

| # | Claim | Check |
| --- | --- | --- |
| 12 | The whole of `verify-quint`, with eight more runs and longer states, finishes inside the CI job's timeout | step 5: time `just quint-traces diff` and the whole recipe against the `timeout-minutes` QM03 set. Not read |
| A | The `tick` that landed is one action that lapses every due lease and then decays everything due, at one `now`, until nothing is due | step 2: read the model at the base and QM09's hand-back; one `just quint run` over a scratch copy, quoting a state before and after a `tick` with two leases due. Read in `.scratch/quint-model/QM09-clock-timestamps-decay.md`, "Design proposed", as a proposal the maintainer had not ruled on; not run |
| B | What the Allium at the base says of the order of lapses due at one `now`: one of the three cases above | step 2: quote the lines. Read at `cf1f3f3` in `docs/specs/pachinko.allium` lines 709 to 730: no order |
| C | The Rust lapses earliest `expires_at` first and breaks a tie by lease id, in one pass, then decays superseded records and then dead letters | step 2: cite T41's hand-back if T41 ran it; otherwise the walk's assertion at `walk.rs` line 520 under `just test`. Read in `expiry.rs` lines 127 to 142 and `leases.rs` lines 106 to 117, not run by this series |
| D | Over the advances that are replayed, the instant the Rust stamps at a step is the model's `now`, although the queue reads its clock lazily | the replay's timestamp comparison on every step, step 6. Read in `queue.rs` lines 276 to 279 and `walk.rs` lines 168 to 173 |
| E | Each shed queue's order by expiry and arrival equals the model's creation order | the replay's state comparison, step 6. An argument, not read in any run |
| F | A witness search reaches each of the seven rules, and the tick that lapses two leases, within the steps and samples ruled | step 2, on a scratch copy of the model; `write` fails, and writes nothing, if one is not reached |
| G | The fixture with timestamps and 32 witness traces is under 768 KB | step 2 measures the scratch set; step 5 measures the committed file |
| H | Which of QM09's three clocks landed, and that the model's clock starts at 1,000. For the two-value clock that refuses a reading behind the latest time, the guard that refuses, quoted beside the Rust's `fn now`. For the two-value clock that holds one, three more: the model's steps that read the clock are the Rust's; its `now` before any read is the Rust's latest read; and its `clock` has the Rust's floor at 0 | step 2: quote the model's `init`, each action's write to `now` and `advance`'s arithmetic, beside `walk.rs` lines 168 to 173, 542 to 546 and 647 to 651. Read in QM09's "Design proposed" and in `walk.rs`, not run |
| I | `run --observation` takes an expression that reads `last`, `prev` and the state, and such a run writes one trace that ends where the expression first holds | step 2, on the scratch copy, for the tick that lapses two leases. Read in `cli.js` (the option's description), not run |

## Non-goals

- No replay of `lapse(lease)`, `decaySuperseded(entry)` or `decayDeadLetter(entry)`.
  They are checked for safety by `just verify-quint` and have no `WalkStep`.
- No replay of a negative advance under a model with one `now`, or under the two-value
  clock that refuses a reading behind the latest time.
- No fixture from `pachinko_zero_ttl`, until T42's ruling says what one tick does there.
- No second tick, no new rule, no new property and no new mutant in the model, and no
  change to `step`, `advance`, `tick` or `lapse`.
- No choice of the order of lapses due at one `now`, in the model, the mapping or the
  Rust.
- No change to `Queue::tick`, `Queue::now` or any file under `crates/libpachinko/src/`.
- No comparison of a refusal's name or of a lease id by value: the tickets T43 creates.
- No new replay by the Python double: QM12 writes it. If the tree this ticket lands on
  holds `double/tests/test_replay.py`, at the base or after the closing merge or
  rebase, shared rule 11 makes its replay part of this change: `advance`, `tick` and
  the timestamps are mapped
  there, and `just python-double` stays green. No step goes on that replay's list
  without a ticket to name (step 7).
- No change to the proptest walk's generator or to what it asserts.

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

`<id>` is `QM11`. Checkpoints:

- Checkpoint 1, in step 3: the plan, before any tracked file changes and before the
  **Stop** for the ruling on "Design proposed".
- Checkpoint 2, in step 8: the diff, before the final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `quint/pachinko.qnt` | shared, own lines | `advance` and `tick` in `replayStep`; nothing else. Created by QM02 |
| `quint/mutants/*.patch` | shared, own lines | only a patch the model change breaks, re-cut with its mutation and its property kept (shared rule 11) |
| `quint/traces/pachinko_walk.json` | shared, own lines | regenerated by `just quint-traces`, never edited by hand. Created by QM10 |
| `quint/traces/manifest.txt` | shared, own lines | regenerated by the same command. Created by QM10 |
| `scripts/quint-traces.py` | shared, own lines | the seven rules in the witness list; the run for a tick that lapses two leases, and the manifest's `cases` key with its place in the count `check` enforces; `advance` and `tick` among the steps a fixture may hold; the numbers, if ruled; `--main=pachinko_walk` unchanged. Created by QM10 |
| `crates/libpachinko/tests/queue/walk/replay.rs` | shared, own lines | the two steps' mapping, the instant's conversion, the tick's comparison, the returned expiries' comparison, the rule test's list of 32 rules and its two-lapse trace, and the list without the rows that name QM11. Created by QM10 |
| `double/tests/test_replay.py` | shared, own lines | only if the tree this ticket lands on holds `double/tests/test_replay.py`, at the base or after the closing merge or rebase: `advance`, `tick` and the timestamps mapped, as QM12's "Time" bullet describes, and the in-suite control on the tick's report that QM12's step 6 describes. Adding a step to that replay's list in place of mapping it needs a ticket to name, and the series has none after this one, so it needs the maintainer's answer first (step 7). Created by QM12 |
| `crates/libpachinko/tests/queue/walk/model/trace.rs` | shared, own lines | timestamps and `consecutive_expiries` read from the state. Created by QM10 |
| `crates/libpachinko/tests/queue/walk.rs` | shared, own lines | `run`'s return value carries the `TickReport`, and a batch's `expires_at` and the instant a renew returns if QM10 did not land them; nothing else |
| `docs/decisions/0022-quint-model-of-the-queue.md` | shared, own lines | conformance: ticks and time are replayed, a bare lapse and a negative advance are not, and why. Created by QM02; if it took another number, that file |
| `docs/reference/testing.md` | shared, own lines | the replay's paragraph: what it now covers and what it still does not |
| `CHANGELOG.md` | shared, own lines | libpachinko, unreleased: the replay covers the clock |
| `.scratch/quint-model/QM11-replay-lapses-and-time.md` | ticket | `status:` and the hand-back notes |
| `.scratch/quint-model/README.md` | shared, own lines | QM11's Status cell |

## Steps

1. **Worktree and baseline.** Confirm QM09 and QM10 have merged into the series base the
   maintainer named. Read T42's hand-back for the tickets it created, and confirm with
   the maintainer that the one that lands the ruled clock in the Rust has merged too;
   if it has not, **stop**: a replay against a Rust that does not hold the ruled clock
   reports that ticket's work as failures. Create the worktree on
   `ticket/qm11-replay-lapses-and-time` from the base and record the base's full commit
   id as `base`. Check that `git ls-files .scratch/quint-model/` lists the README and
   this ticket. Run `just initialize` (network, authorised). Run `just check` on the
   untouched worktree and quote its last line, headed by the commit. Then, after the
   port probe of Verification shows nothing listening on `localhost:8822`, run
   `just verify-quint` and quote its last lines as the baseline.
2. **Read and probe, in scratch only.** Read everything Context names, and note each
   statement there that is no longer true at `base`. Count the variants of `Rule` in
   the model at `base`. This text assumes 32, of which QM10's generator already asks
   for 25. Trace any difference to the T ticket behind it and name it in the plan:
   `tickets/T45-drop-joining.md` removes two rules and the joins that open every
   trace, and then every "32" here reads 30 and every "25" reads 23. Settle claims A,
   B and C, quoting
   the lines and the runs. Copy the model to `ai_tmp/qm11/pachinko.qnt`, add `advance`
   and `tick` to `replayStep` in the copy, and with `just quint run` over the copy
   generate the whole set into `ai_tmp/qm11/sample/`: the time it took, the size
   (claim G), the length of each new witness trace, and any rule not reached (claim F).
   Write `ai_tmp/qm11/plan.md`: `replayStep` before and after; which of the three cases
   of the lapse order holds, with the Allium's lines, and where `last` records a tick's
   order; which of QM09's three clocks landed, the model's `advance` over negative values
   and at a clock of 0, and its first `clock` and `now`, quoted (claim H); the
   conversion of an instant; the expression for the tick that lapses two leases, with
   the run that reached it (claim I); the
   tick's comparison field by field against `TickReport` and against the model's state;
   the list in `replay.rs` before and after; each control of steps 5 and 6; and each
   page's edit. No tracked file has changed.
3. **Adversarial review** (checkpoint 1, `tickets/CONVENTIONS.md` §14: conformance and
   breakage, over `ai_tmp/qm11/plan.md` and `ai_tmp/qm11/sample/`, before any tracked
   file changes). Questions:
   - Does any fixture step map to `WalkStep::Tick` without being the whole due set and
     both decays at one `now`? Can a bare `lapse` or a single-entry decay reach a
     fixture, or a trace of `pachinko_zero_ttl`?
   - Is there a sequence of the advances that are replayed and of steps, refused ones
     included, after which the Rust would stamp an instant other than the model's
     `now`?
   - Does the plan pick an order for lapses due at one `now` that the Allium at the base
     does not give?
   - Is any timestamp field of `Model`, of `TickReport` or of an ended lease neither
     compared with the fixture nor in the list?
   - Could the replay pass with a wrong `expires_at` in a fixture: on a lease, a batch,
     a shed entry or a dead letter?

   **Stop** for the maintainer's ruling on "Design proposed in this ticket", with the
   findings in hand, and record it. The stop is not a formality in four cases, each of
   which needs an answer before step 4: the `tick` that landed is not the whole drain;
   the Allium fixes no order for lapses due at one `now`, fixes one the Rust does not
   follow, or fixes one that the model's `tick` does not record in `last`; the model
   has the two-value clock that holds a reading behind the latest time, and its
   reading steps, its first `now` or its floor are not the Rust's; the model has the
   two-value clock that refuses one, which the Rust does not; or the set does not fit
   under 768 KB, or does not reach a rule or the tick that lapses two leases.
4. **The model.** Add `advance` and `tick` to `replayStep` in `quint/pachinko.qnt`. Run
   `just check-quint` and show it fail on the hash check, naming `just quint-traces`:
   the model changed and the fixtures did not. That failure is the red test for step 5.
   Re-cut any mutant patch the edit broke, keeping its mutation and the property it
   names, and list it in the hand-back (shared rule 11).
5. **The fixtures, red then green.** Extend the generator's rule list and its list of
   steps a fixture may hold, and add the run for a tick that lapses two leases with the
   manifest's `cases` key. Run `just quint-traces`. Quoting each:
   - `just quint-traces check` and `just check-quint` now pass, and
     `just quint-traces diff` passes;
   - with nothing listening on `localhost:8822`, shown by the port probe of
     Verification, `just verify-quint` passes whole: `Canary` violated on every
     instance, every property, every witness and every mutant as at the baseline, and
     the regeneration step. Never pass `--server-endpoint`, and run no `verify`, and no
     `just quint-traces`, which runs the evaluator, while another worktree verifies or
     while `just install-apalache` or `just install-quint-evaluator` runs (shared rule
     10);
   - the fixture holds the trace `tick_lapses_two`, whose last step is a `tick` that
     lapsed two leases, one `tick` that
     decayed an entry and one `advance`; no `lapse` and no single-entry decay; a negative
     advance only if the two-value clock that holds a reading behind the latest time
     landed and was ruled replayable; and no trace
     of any instance but `pachinko_walk`. Say how each was counted, and show the
     counting command find a `pull` first;
   - `just test` **fails**, in the replay, on the first `advance` or `tick` it meets:
     the test does not know the steps yet. Quote the assertion. This is the red test for
     step 6;
   - the time `just quint-traces diff` and the whole of `just verify-quint` took
     (claim 12).
6. **The mapping, green, then its controls.** Extend `replay.rs`, `trace.rs` and
   `run`'s return value, and the rule test: its list becomes the 32 rules, and it
   requires the trace `tick_lapses_two` as "Design proposed" says. Map and remove every
   row of the list that names QM11, and a
   `consecutive_expiries` row that names no later ticket. Run `just test` and show it
   pass, with the number of ticks, lapses and
   decays replayed printed. Then, each on an edit to the fixture restored from a saved
   copy afterwards, show `just test` fail and quote the assertion:
   - one active lease's `expires_at` changed in one state (the control this ticket is
     for);
   - one shed entry's `expires_at` changed;
   - one `advance`'s `millis` in `last` changed;
   - in a tick that decayed an entry, the entry put back in the state after it;
   - in a tick that lapsed a lease, that lease left active in the state after it;
   - where `lapsed` is compared in order (case 1 with the order in `last`, or a
     ruling for case 3 that compares in order), two leases swapped in the order `last`
     records for one tick, which is the field the comparison reads; where it is
     compared as a set, two entries that one tick added to one shed queue swapped in
     the state after it, which the state comparison must catch. Say which applied, or
     that no tick in the fixture added two entries to one queue;
   - the instant a renew returned: in the state after a renew, the lease's `expires_at`
     changed, so that the returned value and the stored one are compared and differ.
     It fails on `answer`, which is compared before the state;
   - the trace `tick_lapses_two` removed, with the manifest's counts and `cases`
     lowered to match (the rule test).

   After the last restore, `cmp` exits 0 for the fixture and for the manifest against
   the copies saved before the first edit. Both hold step 5's uncommitted
   regeneration, so `git diff --exit-code` cannot show the restore. If the
   unedited replay fails, shared rule 2 says which of three things to fix: the mapping,
   here; the model, which is a model change with its own review, so **stop** and ask;
   or the Rust, which is a finding for the maintainer, so **stop**, quote the trace and
   the step, and change neither.
7. **Gates and words.** Run `just fmt-check`, `just clippy`, `just metrics`,
   `just effects` and `just coverage`, and fix each failure at its root with the
   `fix-quality` skill. If the tree this ticket lands on holds
   `double/tests/test_replay.py`, at the base or after the closing merge or rebase, run
   `just python-double` and quote it, red
   first: the double's replay meets steps and fields it does not know. Then map them in
   `double/tests/test_replay.py`, as QM12's "Time" bullet describes, and show it green
   (shared rule 11). QM12 leaves its time controls to the change that makes the
   fixtures exercise them, which is this one: do QM12's step 6 as well. By hand, an
   `expires_at` edited in the fixture fails on the `state` comparison, the fixture
   restored and `cmp` shown. In the suite, the control that wraps `Queue.tick` so that
   its report leaves out one lapsed lease, and in a second case one decayed entry,
   with the state untouched, raises `ReplayMismatch` with comparison `answer`. Quote
   both. Shared rule 11 lets a replay list a step only with the
   maintainer's answer on which ticket maps it, because a listed step ends every trace
   that holds it at the state before, and no ticket of this series comes after this
   one to name. So do not list `advance`
   or `tick` there: if either cannot be mapped, **stop** and ask the maintainer which
   ticket will, and whether this one waits. A divergence of the double from the model
   on a mapped step is QM12's step 5 case: quote it and **stop**. Edit each page in
   Files touched. Run `just check-docs`.
8. **Adversarial review** (checkpoint 2, `tickets/CONVENTIONS.md` §14: conformance and
   breakage, over `git diff "$base"...HEAD` plus the uncommitted changes, before the
   final commit). Questions:
   - Does `replayStep` differ from the base by `advance` and `tick` only, and does
     `step` differ by nothing?
   - Is every row removed from the list now a comparison against the fixture, and is
     any row removed that the model still lacks?
   - Could the controls of step 6 fail for a reason other than the one named, such
     as the hash check or a decoding error?
   - Do the pages say which steps are not replayed (a bare lapse, a single-entry decay,
     and a negative advance under one `now` or under the clock that refuses a reading
     behind the latest time), that `pachinko_zero_ttl` has no fixture
     and why, and that the order of lapses due at one `now` is compared only as far as
     it is ruled?
9. **Commit and hand back.** Fetch, and merge or rebase onto the series base (shared
   rule 13). Read each condition of this ticket again against the merged tree. If it
   now holds `double/tests/test_replay.py` and the base did not, step 7's work on the
   double's replay is owed before the hand-back. If the merge changed
   `quint/pachinko.qnt`, run `just quint-traces` before any gate and rerun the green
   runs of steps 5 to 7. Show the merge's diff at the push stop (the review deltas,
   "After the last checkpoint"). Commit the work. Rerun Verification and quote it,
   headed
   by the full commit id. Record each claim's outcome, the ruling of step 3 and each
   mutant patch re-cut. Set `status: done` here and in the index of
   `.scratch/quint-model/README.md`, write the hand-back notes, and commit the record.
   Then run `just check` once more, on the record's commit, and show its last line and
   an empty `git status --short`: the quoted Verification ran on the commit before, and
   the hook gate has not yet read the notes. If it fails, fix the notes, commit and run
   it again; the work's commit is not amended. **Stop** before pushing or opening a
   pull request: both need authorisation (`tickets/CONVENTIONS.md` §11, "Separately
   authorised actions").

## Acceptance criteria

- `quint/pachinko.qnt` differs from `base` in `replayStep` only, by `advance` and `tick`,
  shown by `git diff "$base"...HEAD -- quint/pachinko.qnt`. `replayStep` offers no
  `lapse`, no `decaySuperseded` and no `decayDeadLetter`, and a negative advance only
  as ruled at step 3.
- Every trace in the fixture comes from `pachinko_walk`; the manifest's `main` says so,
  and no trace comes from `pachinko_zero_ttl`.
- `just verify-quint` passes as shared rule 6 asks, with `Canary` violated on every
  instance and `just quint-traces diff` among its steps; its output is quoted, after the
  port probe.
- The fixture holds a witness trace for each of the 32 rules, the trace
  `tick_lapses_two` ending in a tick that lapsed two leases, and a tick that decayed an
  entry, and is under 768 KB; `the_fixtures_fire_every_replayable_rule` lists 32 rules,
  requires that trace, and passes.
- `just test` passes, and each of step 6's edits makes it fail on the assertion named
  there, the wrong expiry first.
- The list in `replay.rs` has no row that names QM11, and every field of `Model`, of
  `TickReport` and of an ended lease, and every key of a fixture's state, is either
  compared or in it.
- If the tree this ticket landed on holds `double/tests/test_replay.py`, at the base
  or after the closing merge or rebase, `just python-double` passes, its output is
  quoted, and its replay's list names neither `advance` nor `tick`. The two time
  controls of QM12's step 6 are in place and quoted: the edited `expires_at`, and the
  tick's report with a lapse or a decay left out failing on `answer`.
- A batch's `expires_at` and the instant a renew returns are each compared with the
  lease's `expires_at` in the fixture.
- The record's commit passes `just check` too, run after it was made.
- The order of lapses due at one `now` is compared in order exactly where the Allium
  at the base fixes it and `last` records it. Where the Allium fixes it and `last` does
  not, `lapsed` is compared as a set. Where the Allium fixes none, it is compared as
  the maintainer ruled at step 3. The hand-back quotes the Allium's lines, what `last`
  holds for a tick, and the ruling.
- `git diff "$base"...HEAD --name-only` names no file under `crates/libpachinko/src/`.
- Each claim of "Claims this ticket checks" has its outcome recorded.
- Both checkpoints are recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run from a file as one `sh` script, after the commit; `set -eu` stops it at the first
command that fails.

```sh
set -eu
git rev-parse HEAD
mkdir -p ai_tmp/qm11

# Whatever stops the script, the file being edited is put back from its saved copy.
saved=""
target=""
restore() {
  if test -n "$saved"; then cp "$saved" "$target"; fi
}
trap restore EXIT
trap 'exit 1' HUP INT TERM

just quint-traces check
just check-quint
just test

# The fixture holds the clock's steps, and no bare lapse. The control finds a pull.
grep -c -F '"Pull"' quint/traces/pachinko_walk.json
grep -c -F '"Tick"' quint/traces/pachinko_walk.json
grep -c -F '"Advance"' quint/traces/pachinko_walk.json
if grep -E '"(Lapse|DecaySuperseded|DecayDeadLetter)"' quint/traces/pachinko_walk.json; then exit 1; else test "$?" -eq 1; fi
grep -Fx 'main=pachinko_walk' quint/traces/manifest.txt

# The tick that lapses two leases has its own trace; the rule test requires it.
grep -E '^cases=(.* )?tick_lapses_two( .*)?$' quint/traces/manifest.txt
grep -c -F '"tick_lapses_two"' quint/traces/pachinko_walk.json

# A wrong expiry fails the test: the first expires_at in the file becomes 7.
cp quint/traces/pachinko_walk.json ai_tmp/qm11/fixture.saved
saved=ai_tmp/qm11/fixture.saved
target=quint/traces/pachinko_walk.json
awk '!done && sub(/"expires_at":[0-9]+/, "\"expires_at\":7") { done = 1 } { print }' "$saved" >| "$target"
if cmp -s "$saved" "$target"; then exit 1; fi
if just test >| ai_tmp/qm11/red-expiry.log 2>&1; then exit 1; fi
cp "$saved" "$target"
saved=""
grep -E 'replay .* step [0-9]+: [a-z]+:' ai_tmp/qm11/red-expiry.log
git diff --exit-code -- quint/traces
just test

# No file of the library changed. The control shows the diff is read and names the test.
git diff "$base"...HEAD --name-only >| ai_tmp/qm11/names.txt
grep -F 'crates/libpachinko/tests/queue/walk/replay.rs' ai_tmp/qm11/names.txt
if grep -F 'crates/libpachinko/src/' ai_tmp/qm11/names.txt; then exit 1; else test "$?" -eq 1; fi

# The size, under the hook's 768 KB.
size=$(wc -c < quint/traces/pachinko_walk.json | tr -d ' ')
test "$size" -gt 0
test "$size" -lt 786432

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
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

Set `base` in the environment to the commit step 1 recorded before running it; `set -u`
stops the script if it is missing. The `if just test ...; then exit 1; fi` line is
the red test and the `just test` lines before and after it are the green ones. The
`cmp -s` line before it is the positive control for the edit: if the fixture held no
`"expires_at":<digits>`, nothing would change and the script stops there. The `grep -E`
after the red run shows it failed in a comparison of the replay, by the prefix QM10
gave every such message (`replay <trace> step <index>: <comparison>:`); the test's
name alone would also match the line that reports it passing while another test
failed. While `saved` is set, the `trap` puts the fixture back however the script
ends, an interrupt included. `grep -c -F '"Pull"'` is the positive
control for the silent probe of the three model-only steps, the `grep -F` of `replay.rs`
for the silent
probe of `crates/libpachinko/src/`, and the first `lsof`, run while the control listener
holds the port, for the two that must print nothing. `"Pull"`, `"Tick"`, `"Advance"` and
`"Lapse"`, with `"DecaySuperseded"` and `"DecayDeadLetter"`, are the variants of the
model's `Step` as QM02 spells `Pull`; adjust them and
`"expires_at":` to the spelling the fixture uses, and say so under Departures. Step 6
holds the other edits, and step 4 the red run of the
hash check; their output is quoted there.

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

- **The order of lapses due at one `now`.** At `cf1f3f3` the Allium fixes none, T41
  investigates and T42 lands the ruling. If the Allium at this ticket's base still fixes
  none, the maintainer rules at step 3 among these, and this ticket recommends none of
  them. (a) Compare order-free where the order is not fixed: `lapsed` as a set, and the
  entries one tick adds to a shed queue as a multiset. (b) Keep only traces in which no
  tick lapses two leases that each add an entry to the same shed queue, and compare
  `lapsed` as a set. (c) Wait for the specification to fix an order. (d) Keep only
  traces in which the order QM09's `tick` records in `last` is the Rust's, earliest
  `expires_at` first and then earlier grant, and compare in order; that tests the Rust
  against an order the Allium does not give. Each of (a) and (b) leaves the Rust's own
  order checked only by the walk. Until the ruling, `lapsed` is never compared in order
  in this case: `last` holds the order the model chose, the Rust holds its own, and
  both may be legal. If the Allium does fix an order and the model's `tick` at the
  base does not record in `last` the order it took, as QM09's ticket says it does, no
  fixture shows the order: the maintainer rules at step 3 whether QM09 is amended to
  record it, and until then `lapsed` is compared as a set.
- **The ticket that lands the ruled clock in the Rust has no id.** T42 creates it. Until
  it has merged, this ticket is blocked, whatever `depends_on` says. Whoever creates it
  adds its id to this ticket's `depends_on` and to the README's index, as the README's
  "Dependencies" asks for QM09's blocker. Step 1 checks for it either way.
- **A model `now` that is not the Rust's latest read.** Under the two-value clock that
  holds, if the model's first `now` or its reading steps differ from the Rust's, the
  replay's comparison of `now` with `Walk::now` fails whatever advances are replayed:
  at state 0 for a first `now` of 1,000 against the Rust's 0. Replaying forwards only
  does not settle it. The maintainer rules at step 3, before any tracked file changes,
  between amending the model (QM09's, with its own review), a finding against the
  Rust, or a stated comparison in the replay's list; this ticket recommends none and
  never drops `now` from the comparison on its own.
- **A negative advance.** Replayed only if QM09 landed the two-value clock that holds
  a reading behind the latest time, which depends on T42's ruling, and only if the
  model's steps that read the clock, its first `now` and its floor at 0 are the
  Rust's. Under the two-value clock that refuses such a reading it is not replayed,
  and the Rust, which accepts the step, differs from the model: a finding for the
  ticket that lands the ruled clock in the Rust, not something this ticket maps away.
  Whether the Allium says which steps
  read the clock, what the latest read is before any read, or whether an instant can
  go below 0, is not known. The maintainer rules at step 3, with claim H's evidence.
- **`pachinko_zero_ttl`.** No fixture comes from it. With zero TTLs QM09's `tick` drains
  until nothing is due and the Rust's tick is one pass; T41 investigates whether one
  pass leaves anything due, and T42 rules. Once T42's ruling says what one tick does
  there, the maintainer decides whether a later ticket adds fixtures from that instance.
- **QM09's `tick`.** QM09 proposes the whole drain, and its maintainer had not ruled
  when this ticket was written. If the `tick` that landed is not the whole drain, the
  maintainer rules at step 3 whether QM09 is amended or the model gains a tick for
  replay.
- **QM12 in parallel.** This ticket regenerates the fixtures QM12's test reads, and
  adds steps and fields to them. Shared rule 11 puts the double's replay in whichever
  change comes second: it maps `advance`, `tick` and the timestamps there. Listing a
  step in place of mapping it needs a ticket to name and the maintainer's answer
  (step 7). Settled at the second merge.
- **The numbers.** Whether QM10's `witness_steps` and `witness_samples` reach the seven
  rules, and whether the set fits under 768 KB with timestamps, is measured at step 2
  and ruled at step 3.
- **T43 and T45.** `tickets/T43-caller-contract.md` rules on what each action answers,
  the lease's id and the named refusals, and `tickets/T45-drop-joining.md` drops
  joining. The tickets they create change the mapping and the start of every fixture,
  and regenerate the fixtures in the same change. Settled by those tickets when they
  land.
