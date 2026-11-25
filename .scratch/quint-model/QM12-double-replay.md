---
id: QM12
title: "Follow-up: the Python double replays the same traces"
status: todo
depends_on: [QM10]
parallel_with: [QM08, QM09, QM11, QM13]
branch: ticket/qm12-double-replay
estimated_size: M
---

# QM12: Follow-up: the Python double replays the same traces

## Context

After QM10, `quint/traces/` holds trace fixtures generated from the model's `replayStep`,
and a Rust test replays them against the real queue. Each state of a fixture carries the
model's state and three ghosts: `last`, the step taken with its arguments; `answer`, what
the step handed back; and `fired`, the set of Allium rules the step fired, inner rules
included (`.scratch/quint-model/README.md`, "Design fixed before QM01", second block).
The Rust replay compares the state and the answer. It cannot compare `fired`, because
the Rust does not name the rule that ran.

The Python double under `double/` does. It is a second derivation of
`docs/specs/pachinko.allium`, written from the specification alone, and every rule is a
row `Rule(name, guard, effect)` named exactly as the Allium names it
(`double/pachinko_double/rules.py`). Ruling 6 of the series README put its replay after
the Rust's, as this follow-up.

What exists, read at commit `cf1f3f3` (line numbers are as of that commit; cite by name
if they have moved):

- **The facade.** `Queue` in `double/pachinko_double/queue.py` has one method per
  trigger: `publisher_joins` and `consumer_joins` (lines 239 to 243), `publish` (247),
  `pull` (257), `ack` and `nack` (264 to 272), `renew` and `release` (276 to 287) and
  `tick` (291). `snapshot()` (233) returns the whole committed state as one immutable
  `Snapshot` (`double/pachinko_double/inspection.py`, lines 69 to 79): the config,
  `next_position`, and tuples of publishers, consumers, pockets, records, leases,
  superseded records and dead letters. The snapshot keeps ended leases
  (`double/README.md`, line 459).
- **A refusal is an exception.** A trigger no rule accepts raises a subclass of
  `Refusal` (`double/pachinko_double/refusals.py`, line 10) and commits nothing. A
  publisher or consumer id that never joined is refused before the apply step starts
  (`queue.py`, lines 250 to 251 and 339 to 343). `Defect`, `GuardOverlap` and
  `InvariantsBroken` are not refusals: they say the double or the specification is
  wrong.
- **The record of fired rows is cumulative and global.** `dispatch` (`rules.py`, lines
  124 to 146) runs the one row whose guard holds and calls
  `registry.record_fired(table, row.name)` on the module-level registry `SPECIFICATION`
  (line 121). `Registry._fired` (line 93) is a set that only grows, and `fired()` (lines
  113 to 114) returns all of it. `Queue._dispatch` (`queue.py`, lines 380 to 382) passes
  no registry of its own. So nothing today says which rows fired in one step: a
  before-and-after difference misses a rule that had already fired, and clearing the
  set would wipe the record that `double/tests/test_coverage.py` reads last (lines 230
  to 233, "every registered row fired during the run").
- **The seams.** `Queue(config, clock=..., lease_ids=...)` (`queue.py`, line 227) takes a
  `Clock` and a `LeaseIds`. `FakeClock` (`double/pachinko_double/effects.py`, lines 21 to
  45) has `set`, which accepts an earlier time, and `advance`, which raises on a
  negative duration. `SequentialLeaseIds` (lines 48 to 57) answers `lease-1`, `lease-2`
  and so on, and only `PullClaimsFrontPocket` draws one.
- **The answers.** A pull answers `Batch(consumer, lease, pocket, records)`, the records
  oldest first as they stood before the pull, or `NoPocketAvailable(consumer)`
  (`double/pachinko_double/pull.py`, lines 54 and 71). Every other action answers
  nothing. `tick()` answers a `TickReport`.
- **The suite.** `just python-double` (`Justfile`, lines 508 to 516) runs
  `unittest discover -s double/tests -t double` on pixi's Python 3.14, outside
  `just check` and CI. `double/tests/__init__.py` loads every `test*.py` directly under
  `double/tests/`, then `test_coverage` last (lines 73 to 91). `test_coverage.py` fails
  the run if a `def test` under `double/tests/` was not collected or did not pass, a
  skip included (lines 177 to 190), and if a test file sits in a subdirectory (lines 154
  to 158). The double uses the standard library only, with `unittest`
  (`.scratch/python-double/README.md`, lines 11 to 13).
- **The line this ticket amends.** `double/README.md`, line 14, says: "The double is not
  a test oracle for the Rust and is no part of `just check` or CI."

What is missing: nothing checks the double against the model, and nothing checks the
model's `fired` against a second reading of the Allium's guards.

Read these:

- `.scratch/quint-model/README.md`, whole.
- `.scratch/quint-model/QM10-rust-replay.md` (created by the series; QM10 fills its
  hand-back notes): "Design proposed in this ticket" and the hand-back's "Design
  rulings", for the fixture format, the start of a trace and the lease mapping the Rust
  replay uses.
- `quint/traces/` and `quint/pachinko.qnt` (created by QM10 and QM02), at the base.
- The Rust replay test QM10 added under `crates/libpachinko/tests/queue/`, for how it
  maps a step and a state. Read it for the mapping only (shared rule 1).
- `double/README.md`: lines 14 and 16 to 29, and "What stands in for time and for the
  implementation" (from line 415).
- `double/pachinko_double/queue.py`, `rules.py`, `effects.py`, `inspection.py`,
  `model.py`, `pull.py`, `refusals.py` and `config.py`.
- `double/tests/__init__.py`, `double/tests/test_coverage.py` and
  `double/tests/fixtures.py` (`new_queue`, lines 162 to 164), for the suite's shape.
- `crates/libpachinko/tests/queue/walk.rs`: `config()` (lines 59 to 67), `WalkStep`
  (lines 82 to 117), the three joins `walk` prepends (lines 652 to 656) and the payload
  a publish builds (line 307). Names, domains and configuration only.
- `tickets/T41-tick-order-investigation.md`, `tickets/T43-caller-contract.md` and
  `tickets/T45-drop-joining.md`: each changes something this replay maps.
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

`just python-double` replays every fixture under `quint/traces/` against the double. A
new test module, `double/tests/test_replay.py`, reads each fixture with the standard
library's `json`, builds a `Queue`, applies the step each state's `last` names, and
after every step compares three things with the model: the double's snapshot with the
model's state, what the double answered with the model's `answer`, and the names of the
rows that fired in that step with the model's `fired`. A step the model answers with no
rule must be a `Refusal` in the double, and the reverse. The replay fails by name on
anything it cannot map, and never skips.

The double's README says what changed: the double is still not an oracle for the Rust,
and it is now checked against the model's traces. The replay stays in
`just python-double`, outside `just check` and CI.

What the shared rules mean for this ticket. It changes no model and runs no Quint
command, so shared rule 6's `verify` and shared rule 10's port probe do not arise; its
vacuity controls are tests inside the suite, listed under "Design proposed". Shared rule
11 names `just python-double` as a replay every model change keeps green once this
ticket has landed. From then on a ticket that changes the model, QM08, QM09, QM11, QM13
or a T ticket, edits `double/tests/test_replay.py` in its own change: each field or
step the model gains is mapped there, or added to the test's not-compared list with
the ticket that will map it, a step only with the maintainer's answer on which ticket
that is (shared rule 11). This ticket writes that list and the rule that keeps it
honest.

### Owns

| Kind | Names |
| --- | --- |
| Rules | none |
| Invariants | none |
| Guarantees | none |
| Partial or extended, not counted | Every rule the fixtures fire is checked by name against the double's rows. That is a check on `fired`, and on `FiredMatchesGuards` and `UnansweredStepChangesNothing`, which QM02 owns; it adds no property to the model |

## Design proposed in this ticket

These are proposals. The maintainer confirms or changes them after checkpoint 1 and
before step 4, and the hand-back records the ruling. The series README names no design
point for QM12, so every item below is this ticket's own.

- **The fixtures are read as QM10 ruled them.** This ticket is written against the
  layout QM10 recommends (`.scratch/quint-model/QM10-rust-replay.md`, "Design proposed
  in this ticket"): one JSON document, `quint/traces/pachinko_walk.json`, whose `traces`
  list holds every trace with its `name`, `seed` and `states`, and a manifest,
  `quint/traces/manifest.txt`, whose `traces=` line gives their number. The fixtures
  come from `pachinko_walk` only. A state has no
  `prev`; the state before it is the one before it in the list. The number of traces is
  the length of that list, never the number of files, and the test fails if it differs
  from the manifest's. If the maintainer ruled another layout at QM10's step 3, the
  plan follows the ruling and says what changed.
- **One module, found from the repository's root.** `double/tests/test_replay.py`,
  directly under `double/tests/` so that `load_tests` collects it. It finds the fixtures
  as `test_coverage.py` finds the specification (lines 48 to 51): the repository's root
  is two parents above the test file, and the fixtures are `quint/traces/` under it.
  It imports nothing outside the standard library and the double.
- **The rows fired in one step are captured by the test, and the registry's record is
  left alone.** For the length of one replay the test replaces
  `rules.SPECIFICATION.record_fired` with a recorder of its own, through
  `unittest.mock.patch.object` used as a context manager, so that it is restored when a
  replay fails. The recorder appends the row's name to the current step's set when
  `rules.SPECIFICATION.holds(table)` is true, and does not call the real method. The
  coverage test's "every registered row fired during the run" therefore still rests on
  the double's own tests, not on the replay. Alternative: give the package a per-outcome
  record (`Queue` keeps the names its last outcome fired, or `dispatch`'s callers pass a
  registry). That changes `double/pachinko_double/queue.py` or `rules.py`, the names
  `double/tests/test_interface.py` pins and the reader's guide, for something only this
  test reads. Not recommended.
- **The start is compared, not assumed.** The test builds an empty queue, as
  `new_queue` does, and compares it with the fixture's first state, whose `last` is
  `Init`. If that state already holds publishers or consumers (QM10 recommends a start,
  `replayInit`, that has taken the walk's three joins), the test first joins exactly
  those, publishers then consumers, each in name order, and then compares. A first
  state that holds anything else the double cannot reach by joining (a pocket, a
  record, a lease, a shed entry or a `next_position` other than the new queue's own,
  which is 0 today and which `tickets/T47-start-rule-and-config-bounds.md` may change
  in the model and the double together) fails the test and
  names the field. Alternative: assume the start QM10 ruled and compare from the first
  step. Not recommended: T45 changes the start, and an assumed start fails late and
  unclearly.
- **Names and configuration.** Publisher, consumer, pocket and record-key names are the
  model's strings, used as the double's ids and keys unchanged. The config is
  `pachinko_walk`'s: `max_pull_batch` 2, `max_consecutive_expiries` 2, `lease_ttl` 100,
  `superseded_ttl` 150 and `dead_letter_ttl` 250, in milliseconds
  (`.scratch/quint-model/README.md`, second design block; `walk.rs`, lines 59 to 67).
  The fixtures come from that one instance, so the config is one constant in the test.
  A
  payload is built from the publish's tuple by one function, which is also the one that
  reads a payload back for comparison; the form is the one QM10's ruling fixed.
- **A lease is mapped, never guessed.** The model's lease identity is QM02's ruling; the
  double's is the string `SequentialLeaseIds` draws. The test keeps, per trace, a map
  from the model's lease to the double's id. An entry is added when a pull answers a
  `Batch` under a lease the map does not hold, pairing the lease the model's `answer`
  names with `Batch.lease`, and it is kept after the lease ends, so that a model step
  that names an ended lease names the same ended lease in the double. The map is
  brought up to date before the state is compared, since the state comparison reads
  it; if one side answered a batch and the other did not, that is an `answer` mismatch,
  raised at once. The map must stay one to one; the test fails if two model leases meet
  one id. A model step that names a
  lease the map does not hold (one never issued, if QM02's domain has such a value) is
  given an id the double never drew, and both sides must refuse it. Alternative: a
  `LeaseIds` in the test that hands the double the model's own identity, read from the
  next `answer`, so that the two states compare with no map. It is shorter, and it
  feeds the model's output into the double; the map is what QM10's Rust replay uses.
  Not recommended.
- **What is compared after every step, in this order.**
  1. *Refusal.* For `publisherJoins`, `consumerJoins`, `publish`, `pull`, `ack`, `nack`,
     `renew` and `release`: the double raised a `Refusal` exactly when the model's
     `fired` is empty, and after a refusal the double's snapshot equals its snapshot
     before the step. The refusal's class is not compared with anything: the model has
     no named refusal until T43's tickets land. Only `Refusal` is caught. `Defect`,
     `GuardOverlap`, `InvariantsBroken` and every other exception fail the test with
     the fixture and the step. `advance` and `tick` never refuse.
  2. *State.* The double's snapshot against the model's state, through one projection.
     Publishers, consumers, pockets, records and leases are compared as maps by key,
     since the snapshot's listing order is not a claim about the specification
     (`double/README.md`, lines 459 to 470). The two shed queues are compared as lists,
     in order: the model keeps them in creation order and so does the double
     (`queue.py`, lines 62 to 63 and 192 to 195). The entries that two lapses of one
     tick add to one shed queue stand in the order of those lapses, so they are
     compared in order only where the tick's `lapsed` is ("Time", "What a tick
     answers"), and otherwise as that same ruling says. Leases are compared in full, ended
     ones included, through the lease map. What is not compared is named in one list
     in the test, the not-compared list, as shared rule 11 asks: each entry is a field
     or a step, with the ticket that will map it. With QM10 alone in the base the list
     is the snapshot's timestamps, which the model has none of before QM09:
     `granted_at`, `expires_at` and `ended_at` of a lease, `nacked_at` of a record,
     `shed_at` and `expires_at` of a superseded record, and `nacked_at` and
     `expires_at` of a dead letter, each naming QM09. If the base's model has no
     `consecutive_expiries`, which is so until QM08 lands, the list also holds a
     pocket's `consecutive_expiries`, naming QM08, and the replay expects 0 of the
     double there, as QM10's Rust replay expects of the Rust: no fixture holds a
     lapse, so the double's count never leaves 0, and a value that did is a mismatch
     and not a gap. QM08 removes the entry when it maps the field. The list cannot go
     stale silently: a field of a fixture's state, or of the snapshot, that is neither
     compared nor listed fails the test and names the field.
  3. *Answer.* A `Batch` against the model's batch: the lease through the map, the
     pocket, and the records in order, each by key, rank and payload. `NoPocketAvailable`
     against the model's. `None` against nothing. A `TickReport`, which `tick()` always
     answers, as "Time" below says.
  4. *Fired.* The step's set of row names against the model's `fired`, as sets. Every
     name in a fixture's `fired` must be in `rules.SPECIFICATION.rule_names()`.
- **A mismatch names its comparison.** The replay raises one exception type,
  `ReplayMismatch`, with the fixture's name, the step's index, the comparison (`start`,
  `refusal`, `state`, `answer` or `fired`) and the two values. Its message begins
  `<fixture> step <index>: <comparison>:`. Because the comparisons run in the order
  above, a mismatch whose comparison is `fired` says that the refusal, the state and the
  answer agreed at that step and at every step before it.
- **Time: three cases, keyed on what the fixtures hold.** The double has a clock
  whatever the fixtures hold, and QM09 can merge without QM11.
  - *No `now` in any state* (neither QM09 nor QM11 is in the base). The clock is
    `FakeClock(0)` and is never moved, and `tick()` is never called, so no lease lapses
    and nothing decays. The timestamps are in the not-compared list, naming QM09.
  - *`now` and timestamps in the state, and no `advance` or `tick` step* (QM09 is in the
    base and QM11 is not). The clock starts at the first state's clock value and is
    never moved. The timestamps leave the not-compared list and are compared.
  - *`advance` or `tick` among the steps* (QM11 is in the base). As the case before,
    and `advance(millis)` moves the `FakeClock` by `millis`: `FakeClock.advance` for a
    value that is not negative, and for a negative one whatever QM09's ruling says the
    model's clock does, through `FakeClock.set`, since `advance` refuses a negative
    duration. `tick` is `Queue.tick()`, and the rows compared are the union over every
    outcome of that one call.

  Which value of the model is the clock's depends on which of QM09's three clocks
  landed (QM09, "How the clock moves"), and step 2 reads it. With one `now`, the clock
  starts at the first state's `now`, and after
  every step the model's `now` is compared with the clock's time; the snapshot has no
  `now`, so a difference there is a `state` mismatch. With two values, `clock`, which
  an advance moves, and `now`, the latest instant a step read, the `FakeClock` is the
  model's `clock`: it starts at the first state's `clock` and is compared with it
  after every step. The model's `now` then has no counterpart. The two values come in
  two clocks. One holds a reading behind the latest time, and the rest of this
  paragraph is about it. The other refuses one: a rule that reads the clock answers
  only when `clock` is at or after `now`. Under that clock QM11 puts no negative
  `advance` in `replayStep`, so no fixture steps the clock back, the refusal never
  applies in a trace, and the replay needs nothing more for it. The double would
  accept a step that model refuses, since it reads its clock raw; that is a finding
  for the ticket of `tickets/T42-clock-model.md`'s follow-ups that lands the ruled
  clock in the double, written under Open points, and no fixture shows it.

  What the double does with its clock is read at the base, in step 2, and not assumed
  from this text: `tickets/T42-clock-model.md` lets its follow-up land the ruled clock
  in the double before this ticket runs. Read at `cf1f3f3`, not run: the
  double reads its clock raw, once for an outcome (`_Now` in
  `double/pachinko_double/queue.py`, and `tick`), and keeps no latest instant between
  steps, so after an advance back it stamps an earlier time than a model that holds
  the latest read. If the double at the base keeps the latest instant, where its
  snapshot or its queue shows it, the model's `now` is compared with that value after
  every step, nothing about the clock is listed, and the rest of this paragraph and
  the Open point "The double has no latest instant" do not apply. If it still keeps
  none, recommended: the test does not hide that. It compares every
  timestamp the state holds, lists the model's `now` in the not-compared list as a
  value the double does not keep, naming the ticket of
  `tickets/T42-clock-model.md`'s follow-ups that lands the ruled clock in the double,
  which step 2 has shown is not in the base,
  and lets a stamp that differs fail as a `state` mismatch. That mismatch is step 5's
  second cause, the double differs from the Allium: quote it and **stop**. Alternative:
  the test keeps the latest read itself and sets the `FakeClock` to it before each
  step. The replay would pass, by feeding the model's clock into the double, and it
  would hide that the double has no such clamp. Not recommended. The maintainer rules
  at the **Stop** in step 4.

  What a tick answers. `Queue.tick()` returns a `TickReport` whatever the model's
  `answer` holds after a tick, which may be nothing (QM09, "What `answer` holds after a
  tick"). The report is compared with the two states, and a difference is an `answer`
  mismatch: `lapsed` against the leases whose status went from active to expired
  between the state before and the state after, each in full through the lease map,
  and in order only where QM11's rule compares it in order (QM11, "What is compared
  for a tick"): where the Allium at the base fixes the order and `last` records it,
  or, where the Allium fixes none, as the maintainer ruled at QM11's step 3, read
  under "Design rulings" in the hand-back of
  `.scratch/quint-model/QM11-replay-lapses-and-time.md`. An order
  in `last` is not enough by itself: where the Allium fixes none, `last` holds the
  order the model chose, the double holds its own, and both may be legal.
  `decayed_superseded` and `decayed_dead_letters` are compared with the entries that
  left each shed queue between the two states, entry by entry in the queue's order. If
  the model's `answer` after a tick holds something, the report is compared with that
  as well.

  The ticket needs only QM10. Steps 5 and 6 do the part of this that the base's
  fixtures exercise. The rest lands with the ticket that makes the fixtures exercise
  it, QM09 or QM11, which edits this test in its own change under shared rule 11.
- **A step with no mapping fails by name, unless the list names it.** The test maps the
  `last` variants `Init`, `publisherJoins`, `consumerJoins`, `publish`, `pull`, `ack`,
  `nack`, `renew`, `release`, and, when the fixtures hold them, `advance` and `tick`,
  which step 5 maps. A variant that is neither mapped nor in the not-compared list,
  `lapse` included, fails the test with the fixture, the step's index and the
  variant's name. A variant the list names ends the replay of its trace at the state
  before it, since a step that was not applied leaves nothing to compare after it.
  That is a loss, and the test never hides it. Every trace is replayed and compared up
  to its end or to the state before a listed step. The test fails if any trace is cut
  short by a variant the list does not name, if a trace holds one state and no step,
  and if no trace is replayed to its end. On every run it prints one line, on standard
  output, which the recipe does not buffer away (`Justfile`, `python-double`: no `-b`),
  `replay: <n> traces, <n> to the end, <n> cut short`, followed by the word `at` and
  the variants when the last number is not 0, and a control checks that line against a
  set built in memory. When this ticket hands back, the
  list names no step, so every trace is replayed to its end: the mapping covers every
  variant the base's fixtures hold, `advance` and `tick` included if they are there.
  A later ticket that lists a step makes the line say so. The test also fails on a
  missing `quint/traces/`, a fixture set with no trace in it, and a state without
  `last`, `answer` or `fired`. Nothing is skipped: `test_coverage.py` fails a skipped
  test anyway (lines 177 to 190).
- **Vacuity controls live in the suite.** Each builds its input in memory from the
  fixture and runs on every `just python-double`. The step they edit is chosen by a
  stated rule, since the first step that fires a rule may be a join, with no pocket
  and no record to edit: the first step, in the first trace that has one, whose
  `fired` is not empty and whose state holds a record. Each control fails, by name,
  if no such step exists or if its edit left the value as it was.
  - an edited state (that record's rank, moved to another rank of the domain) raises
    `ReplayMismatch` with comparison `state`;
  - an edited `fired` set at the same step, one name replaced by a registered rule's
    name that the set does not hold, with the state and the answer untouched, raises
    `ReplayMismatch` with comparison `fired`; a second case adds such a name. Neither
    empties the set, which would trip the refusal comparison first;
  - a step whose `last` is an unknown variant fails and names it, and a state with a
    field that is neither compared nor listed fails and names it;
  - a fixture set with no trace fails, and so does a trace of one state;
  - a set in which one trace holds a step the list names, built by adding the variant
    to a copy of the list, is replayed up to that step, and the printed line counts it
    as cut short; the same set with the variant not listed fails;
  - after a replay, `rules.SPECIFICATION.fired()` is what it was before.
- **The gate does not move.** The replay runs in `just python-double`, outside
  `just check` and CI, as the series README says. Alternative: the `quint` CI job runs
  `just python-double` after `just verify-quint`. That is the maintainer's to decide
  (Open points).

## Claims this ticket checks

No row of the series README's "Claims the tickets check" names QM12. The claims below
are the ones this ticket rests on and nobody has run.

| # | Claim | Check |
| --- | --- | --- |
| A | Every state of every fixture QM10 committed carries `last`, `answer`, `fired` and the model's state, in a form `json.load` reads with no third-party help. Not read: `quint/traces/` did not exist when this ticket was written | step 2: load every fixture and list the keys of its first and last state |
| B | Replacing `record_fired` on the instance `rules.SPECIFICATION` intercepts every row that fires through `Queue`. Read in `double/pachinko_double/rules.py` (lines 124 and 145: `dispatch`'s default registry is that same object), not run | step 4: one pull that hands out a record yields `PullClaimsFrontPocket` and `HoldRecord` in the step's set, and `rules.SPECIFICATION.fired()` is unchanged |
| C | The model's state keeps ended leases, as the double's snapshot does, so leases compare in full. Read in `.scratch/quint-model/README.md` ("Leases are entities with an identity of their own, ended ones included") and `double/README.md` line 459; the fixtures not read | step 2: a fixture with a release shows the ended lease in a later state |
| D | The model's positions are numbers allotted from a counter that starts at 0, as the double's are (`queue.py`, line 70). Not read: how the pocket order is written is QM02's ruling | step 2. If the model writes the order another way, the projection compares the order the positions give, and the plan says so |
| E | With no `tick()` and an unmoved clock, no rule of the double reads a due time, so a fixture without time replays whatever the TTLs are. Read in `queue.py` (lines 291 to 326: a lapse and a decay happen only in `tick`), not run | step 5: the green run |

## Non-goals

- No change to `quint/pachinko.qnt`, to `quint/traces/`, to how the fixtures are
  generated or to the Rust replay.
- No change to the double's rules, guards or effects. A divergence is reported, not
  repaired here (step 5).
- The double does not become an oracle for the Rust, and no Rust test reads it.
- No place in `just check` or CI.
- No comparison of a refusal's name. T43's tickets add named refusals to the model, and
  the comparison with them.
- No replay of the model's `lapse` steps. They are in `step` alone and no fixture holds
  one.
- No third-party package, and no `pytest`.

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

`<id>` is `QM12`. Checkpoints:

- Checkpoint 1, the plan: step 3, before any tracked file changes.
- Checkpoint 2, the diff: step 8, before the final commit.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `double/tests/test_replay.py` | new | the fixture reader, the mapping from `last` to a `Queue` call, the projection with its one not-compared list, the lease map, the per-step recorder, `ReplayMismatch`, the replay test and the vacuity controls |
| `double/README.md` | shared, own lines | line 14, as "Goal" says and step 7 words it; the paragraph under "Running it" that lists what the suite holds; one sentence saying that a change to the model or the fixtures reruns `just python-double` |
| `docs/reference/commands.md` | shared, own lines | the `just python-double` row: the suite also replays the fixtures under `quint/traces/` |
| `docs/project/repository-map.md` | shared, own lines | only if the row QM10 wrote for `quint/traces/` names what reads the fixtures |
| `docs/decisions/0022-<slug>.md` | shared, own lines | created by QM02, which chooses the slug: only if the record lists what replays the traces, one sentence for the double's replay |
| `.scratch/quint-model/QM12-double-replay.md` | ticket | `status:` and the hand-back notes |
| `.scratch/quint-model/README.md` | shared, own lines | QM12's Status cell; and, only if the maintainer's ruling on "Design proposed" differs from a recommended answer, the sentences it changes there |
| `.scratch/quint-model/QM08-*.md`, `QM09-*.md`, `QM11-*.md`, `QM13-*.md` | shared, own lines | only if that ruling differs from a recommended answer: the sentences it changes, each listed in the hand-back |
| `tickets/T39-decide-q2-q6.md`, `tickets/T40-pocket-order-model.md`, `tickets/T42-clock-model.md`, `tickets/T43-caller-contract.md`, `tickets/T45-drop-joining.md`, `tickets/T46-no-claim-without-records.md`, `tickets/T47-start-rule-and-config-bounds.md` | shared, own lines | only if that ruling differs from a recommended answer, and only in a ticket still `todo`: the paths and commands that its "The Quint model" paragraph and its `quint/pachinko.qnt` row take from this ticket, each listed in the hand-back |

If decision 0022's number is not 0022, read "Before QM01 starts" in the series README;
the hand-back names the file as it is.

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/qm12-double-replay` from
   the series base the maintainer names, once QM10 has merged there. Record the base's
   full commit id as `base`. Confirm `git ls-files .scratch/quint-model/` lists the
   series README and this ticket. Run `just initialize` (network, authorised). Run
   `just check` on the untouched worktree and quote its last line, headed by the commit.
   Run `just python-double` and quote its `Ran <n> tests` line and its `OK`, headed by
   the commit: that count is the baseline the new tests add to.
2. **Read before writing.** No tracked file changes in this step. Read the list under
   Context. Then, with pixi's Python (`.pixi/envs/default/bin/python -B`) and a scratch
   script under `ai_tmp/qm12/`, load the fixtures under `quint/traces/` and record: how
   many traces there are, the keys of the first and the last state of each, the set of
   `last` variants across all of them, the union of every `fired`, and whether any
   state holds `now` (claims A, C and D). Read the other side of the mapping too, at
   the base: how the double reads its clock in `double/pachinko_double/queue.py`,
   whether it keeps a latest instant between steps, and which of T42's follow-up
   tickets have merged. "Design proposed" describes the double as it was at
   `cf1f3f3`. **Stop** and ask if a state lacks `last`,
   `answer` or `fired`, or if `quint/traces/` holds no trace: QM10's ruling is then not
   what this ticket was written against. Write `ai_tmp/qm12/plan.md` with:
   - the mapping table: each `last` variant, its arguments as the fixture spells them,
     and the `Queue` call with its arguments;
   - the projection: each field of the model's state beside the snapshot field it is
     compared with, and the not-compared list with the ticket that will map each
     entry;
   - how `answer` and `fired` are spelt in the fixture and what each is compared with;
   - the lease map's rule, with the model's lease identity as QM02 ruled it;
   - which of the three time cases the base is in, with the evidence, and, if a state
     holds `now`, which of QM09's three clocks the model has, and whether the double
     at the base keeps a latest instant, with the lines that show it;
   - whether the base's model has `consecutive_expiries`, and so whether the
     not-compared list holds it;
   - each test's name and what makes it fail;
   - the new wording for each page in Files touched.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qm12/plan.md` with step 2's listing, before any tracked file changes;
   `tickets/CONVENTIONS.md` §14 with the deltas above). Questions:
   - Can the replay pass while the double and the model disagree: a field in neither
     the projection nor the not-compared list, a collection compared as a set where
     order
     is the claim, or a lease compared through a map that hides a mix-up?
   - Does the recorder see every row that fires through `Queue`, inner rules included,
     and can a replay add to or clear the record `test_coverage.py` reads?
   - Is there a step for which "the model fired nothing" and "the double raised a
     `Refusal`" are not the same statement, and what does the plan do there?
   - Can the test pass with no fixture, with a fixture of one state, or with a step it
     has no mapping for?
   - Does anything in the plan read the double to decide what the model should do, or
     the model to decide what the double should do?
4. **Red: the controls first.** **Stop** first for the maintainer's ruling on "Design
   proposed", with the checkpoint's findings in hand, and record it. Then write
   `double/tests/test_replay.py` with the reader, the recorder, `ReplayMismatch` and the
   vacuity controls, and with a replay that maps `Init` only. Run `just python-double`
   and quote the failure: the replay test fails by name on the first step it has no
   mapping for, and the unknown-step control and the empty-directory control pass.
   Show claim B with the recorder's own test.
5. **Green: the replay.** Add the mapping, the projection, the lease map and the four
   comparisons, for everything the base's fixtures hold. If step 2 found `now` in a
   state, that includes starting the clock from the first state and comparing the
   timestamps, which leave the not-compared list. If it found `advance` or `tick`
   among the `last` variants, it includes their two mappings and the tick's report
   ("Design proposed", "Time"). The replay cannot be green before them: a trace that
   holds a step with no mapping fails by name, and no step is listed to get past that.
   Run `just python-double` and quote `Ran <n> tests`, `OK` and the replay's printed
   line, headed by
   the commit and "with uncommitted changes". A mismatch here has one of four causes,
   and the hand-back says which for each one met:
   - the test maps a step or a field wrongly: fix the test, against QM10's ruling;
   - the double differs from the Allium: quote the rule, the fixture, the step and
     both values under Open points. This ticket does not edit `double/pachinko_double/`;
   - the model differs from the Allium: the same, and this ticket does not edit the
     model;
   - the Allium is silent or wrong: **Stop** and ask the maintainer (shared rule 2).

   For the second and third, **Stop** and ask whether to hand back with the replay
   red, or to wait for the fix. Never add the diverging field to the not-compared list,
   and never narrow a comparison, to make the run pass.

   Then two red tests by hand, on the committed files. Save one fixture to
   `ai_tmp/qm12/fixture.saved`. Write `ai_tmp/qm12/edit_fixture.py`, a scratch script
   with three modes. Mode `pick` prints the path of the fixture file the other two
   edit, and changes nothing. Mode `state` changes one record's rank, and mode `fired`
   replaces one name of a `fired` set with a registered rule's name the set does not
   hold; both act at the step the in-suite controls use (the first, in the first trace
   that has one, whose `fired` is not empty and whose state holds a record), print the
   trace, the step and the old and new values, and exit non-zero, writing nothing, if
   there is no such step or the value would not change. Run each edit, run
   `just python-double`, quote the failing
   `<fixture> step <index>: <comparison>:` line and the exit status, and restore the
   fixture. After the last restore, `cmp` of the fixture and the saved copy exits 0
   and `git diff --exit-code quint/traces` exits 0.
6. **Time's controls** (only if step 2 found `now` in a state, or `advance` or `tick`
   among the `last` variants; step 5 has already mapped them). One more red test by
   hand, as in step 5: an `expires_at` edited in a fixture fails on the `state`
   comparison. If the steps hold `tick`, the suite gains one more control, because no
   edit to a fixture reaches the report's comparison: the state is compared first and
   fails first. For the length of one replay, `Queue.tick` is wrapped so that the
   report it returns leaves out one lapsed lease, and in a second case one decayed
   entry, with the state untouched; each raises `ReplayMismatch` with comparison
   `answer`. The control fails by name if no fixture holds a tick that lapsed a lease.
   A mismatch, met in step 5 or here, that turns on which of two
   lapses due at one `now` runs first is not this ticket's to settle: quote it under
   Open points with what `tickets/T41-tick-order-investigation.md` had ruled by then,
   and **Stop** and ask. Write under Departures which parts of this step did not run
   because the fixtures did not exercise them; the ticket that makes the fixtures
   exercise them does those parts in its own change.
7. **Words.** In `double/README.md`, line 14 becomes two sentences, in these words
   unless checkpoint 2 improves them: "The double is not a test oracle for the Rust and
   is no part of `just check` or CI. It is checked against the Quint model:
   `tests/test_replay.py` replays the model's traces under `quint/traces/` and
   compares the state, the answer and the rules fired after every step." Add the replay
   to the paragraph under "Running it" that lists what the suite holds, with the
   sentence that a change to `quint/pachinko.qnt` or to the fixtures reruns
   `just python-double`. Edit the `just python-double` row in
   `docs/reference/commands.md`, and `docs/project/repository-map.md` and decision
   0022 if either lists what reads or replays the fixtures; the `review-docs` skill
   applies to all three. Run `just check-docs`.
8. **Adversarial review** (checkpoint 2: conformance and breakage, over
   `git diff "$base"...HEAD` plus the uncommitted changes, with the output of steps 4 to
   6, before the final commit; `tickets/CONVENTIONS.md` §14 with the deltas above).
   Questions:
   - Would the `fired` control still pass if the replay never compared `fired`, or
     compared it only when the state differed?
   - Does any path through the test catch more than `Refusal`, or swallow a mismatch?
   - Does every entry of the not-compared list name the ticket that will map it, and
     does a field that is neither compared nor listed fail the test?
   - Does `just python-double` fail, by name, when `quint/traces/` is missing, empty or
     holds a step the test cannot map?
   - Do the pages say only what was shown: checked against the model's traces, not an
     oracle for the Rust, not in `just check`?
   - Does the suite still import nothing outside the standard library and the double?
9. **Commit and hand back.** Confirm every writer has stopped. Fetch, and merge or
   rebase onto the series base (shared rule 13). Read each condition of this ticket
   again against the merged tree: if the merge changed `quint/traces/`, because QM08,
   QM09, QM11 or a T ticket landed meanwhile, rerun step 2's listing, and do what
   steps 5 and 6 owe for what the fixtures now hold (a field that left or joined the
   not-compared list, `consecutive_expiries` among them, the time case, a new step).
   If the merge changed anything under `double/pachinko_double/`, because a follow-up
   of `tickets/T42-clock-model.md` or another T ticket landed meanwhile, read again
   how the double reads its clock and whether it keeps a latest instant, as step 2
   does, whether or not a fixture changed. An entry of the not-compared list that
   names a ticket the merged tree now holds is mapped and removed, the model's `now`
   among them, and steps 5 and 6 are done again for it.
   Show the merge's diff at the push stop (the review deltas, "After the last
   checkpoint"). Run `just check` and
   `just python-double`. Commit the work. Rerun Verification and quote it, headed by
   the full commit id. Set `status: done` here and in `.scratch/quint-model/README.md`,
   write the hand-back notes, and commit the record. Then run `just check` once more,
   on the record's commit, and show its last line and an empty `git status --short`:
   the quoted Verification ran on the commit before, and the hook gate has not yet
   read the notes. If it fails, fix the notes, commit and run it again; the work's
   commit is not amended. **Stop** before pushing or opening a pull request: both need
   authorisation (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `just python-double` exits 0 and its `Ran <n> tests` count is the baseline's plus the
  tests this ticket adds, both quoted.
- `double/tests/test_replay.py` replays every trace under `quint/traces/` to its end:
  the not-compared list names no step, the replay's printed line says
  `0 cut short`, and the hand-back quotes it with the number of steps compared.
- Every `last` variant step 2 found has a row in the mapping table, and the hand-back
  lists the union of rule names the fixtures fired and the rules no fixture fired.
- The not-compared list is one list, each entry naming the ticket that will map it. A
  field of a fixture's state or of the snapshot that is neither compared nor listed
  fails the test, and so does a step that is neither mapped nor listed; a control in
  the suite shows each. The list holds no timestamp if step 2 found `now` in a state.
  If the fixtures hold `tick`, its `TickReport` is compared as "Time" says, and the
  control of step 6 shows a report that leaves out a lapse or a decay failing on
  `answer`.
- The in-suite controls exist under the names the plan gave them and pass: an edited
  state fails on `state`, an edited `fired` set fails on `fired`, an unknown step, a
  fixture set with no trace and a trace of one state fail by name, a listed step is
  counted as cut short, and a replay leaves `rules.SPECIFICATION.fired()` as it was.
- The red tests of steps 5 and 6 are quoted, each with its failing line and exit
  status, and the fixtures are byte for byte as committed afterwards.
- No file under `double/pachinko_double/`, `quint/` or `crates/` differs from `base`:
  `git diff --stat "$base"...HEAD` lists only the paths in Files touched.
- `double/README.md` no longer says only that the double is not a test oracle: it says
  what the double is checked against, and still says it is no part of `just check` or
  CI.
- Each of claims A to E has its outcome recorded, and each divergence met in step 5 or
  6 has its cause recorded.
- Both checkpoints are recorded as `tickets/CONVENTIONS.md` §14's Record asks.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record's
  commit.

## Verification

Run from a file as one `sh` script, from the repository's root, after the commit;
`set -eu` stops it at the first command that fails. It needs `ai_tmp/qm12/edit_fixture.py`
from step 5. It names the fixture and the manifest as QM10 recommends them; if the
ruling differs, the two lines that read them are corrected and Departures says so.

```sh
set -eu
git rev-parse HEAD
mkdir -p ai_tmp/qm12
py=.pixi/envs/default/bin/python
test -x "$py"

# The fixtures exist, and the suite is green with them.
test -s quint/traces/pachinko_walk.json
traces=$(sed -n 's/^traces=//p' quint/traces/manifest.txt)
test "$traces" -gt 0
just python-double >| ai_tmp/qm12/green.log 2>&1
grep -E '^Ran [0-9]+ tests' ai_tmp/qm12/green.log
grep -Fx 'OK' ai_tmp/qm12/green.log

# The controls are defined; test_coverage fails a def that did not run and pass.
grep -c 'def test' double/tests/test_replay.py
grep -n 'ReplayMismatch' double/tests/test_replay.py

# Whatever stops the script, the fixture being edited is put back from its saved copy.
saved=""
target=""
restore() {
  if test -n "$saved"; then cp "$saved" "$target"; fi
}
trap restore EXIT
trap 'exit 1' HUP INT TERM

# Red: a state edited on disk fails on the state comparison.
target=$("$py" -B ai_tmp/qm12/edit_fixture.py pick)
test -f "$target"
cp "$target" ai_tmp/qm12/fixture.saved
saved=ai_tmp/qm12/fixture.saved
"$py" -B ai_tmp/qm12/edit_fixture.py state "$target"
if cmp -s "$saved" "$target"; then exit 1; fi
if just python-double >| ai_tmp/qm12/red-state.log 2>&1; then exit 1; fi
cp "$saved" "$target"
grep -E 'step [0-9]+: state:' ai_tmp/qm12/red-state.log

# Red: a fired set edited on disk fails on the fired comparison, and on no other.
"$py" -B ai_tmp/qm12/edit_fixture.py fired "$target"
if cmp -s "$saved" "$target"; then exit 1; fi
if just python-double >| ai_tmp/qm12/red-fired.log 2>&1; then exit 1; fi
cp "$saved" "$target"
grep -E 'step [0-9]+: fired:' ai_tmp/qm12/red-fired.log
if grep -E 'step [0-9]+: (start|refusal|state|answer):' ai_tmp/qm12/red-fired.log; then exit 1; else test "$?" -eq 1; fi

# The fixtures are as committed, and nothing outside Files touched moved.
cmp "$target" "$saved"
saved=""
git diff --exit-code quint/traces
just python-double >| ai_tmp/qm12/green-again.log 2>&1
grep -Fx 'OK' ai_tmp/qm12/green-again.log
grep -E 'replay: [0-9]+ traces, [0-9]+ to the end, 0 cut short' ai_tmp/qm12/green-again.log
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The two runs after an edit are the red tests; each must fail. While `saved` is set,
the `trap` puts the fixture back however the script ends: a red run that passed, an
editor that failed after it wrote, or an interrupt. Each `cmp -s` is the positive
control for the edit before it. The first `just python-double` and the one
after the restores are the green tests, and the last shows the replay's own line: no
trace cut short. The probe that passes by printing nothing is
the `grep -E` for the other four comparisons in `red-fired.log`; its positive control is
the same pattern's `state` alternative matching in `red-state.log` two blocks above, and
the `fired` line matched just before it shows the log is not empty. `git diff
--exit-code` runs after the commit, so it compares the committed fixtures. If step 6
ran, add its `expires_at` red test in the same form and say so under Departures.

## Hand-back notes

### Base and branch

### Baseline

### Claims

### Design rulings

### Checkpoint 1: the plan

### Checkpoint 2: the diff

### Departures

### Verification

## Open points

- **A later model ticket that does not list this test.** Shared rule 11 has every
  ticket that changes the model keep `just python-double` green once this ticket has
  landed, and list the replays in its Files touched. A ticket written before this one
  merged may not list `double/tests/test_replay.py`. Its agent hands that back and the
  maintainer adds the row. T41, T45, T46 and T47 each run `just python-double` in their
  verification, so a red suite blocks them meanwhile.
- **Which lapse runs first in a tick.** The double breaks a tie between two lapses due
  at one `now` by the order the leases are listed in (`double/README.md`, lines 466 to
  469), and `tickets/T41-tick-order-investigation.md` investigates it. The fired set
  does not depend on the order; the order of the shed queues' entries can. The
  maintainer settles it through T41 and QM09, before step 5 maps a tick.
- **The double has no latest instant.** This holds only if step 2 finds the double at
  the base as it was at `cf1f3f3`; if T42's follow-up has given it the ruled clock,
  the point is closed and the hand-back says so. If QM09 landed either two-value clock, the
  double differs from it once the clock steps back: under the one that holds, the model
  stamps from the latest instant a step read, and the double stamps from its clock as
  it stands (`_Now` and `tick` in `double/pachinko_double/queue.py`); under the one
  that refuses, the model answers no rule and the double accepts the step. Fixtures
  show the first only, since QM11 replays no advance back under the second. This
  ticket's test shows it and does not repair it: the
  double's fix belongs to the tickets `tickets/T42-clock-model.md` creates. The
  maintainer rules at step 4 whether this ticket waits for that fix, hands back with
  the replay red, or runs before QM11 gives the fixtures a backward advance.
- **Whether anything gates the double's replay.** Shared rule 11 has a model change
  keep `just python-double` green, and no gate and no CI job runs the recipe. The
  maintainer decides at the ruling in step 4 whether the `quint` CI job runs it.
- **Unjoined publishers and consumers.** The double refuses an id that never joined
  before any rule runs (`queue.py`, lines 250 to 251 and 339 to 343), an implementation
  choice its notes record. If the model answers such a step with a rule, step 5 meets it
  as a divergence. The maintainer settles it there; T45 removes joining altogether.
- **Rules no fixture fires.** The replay checks a rule's name only where a fixture
  fires it. The hand-back lists the rules no fixture fired. Whether QM10's seed and
  trace count should be raised to cover them is QM10's ruling to revisit, by the
  maintainer.
