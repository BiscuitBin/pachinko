# The Gleam double: a reader's guide

This directory holds a second double of the queue that
[`docs/specs/pachinko.allium`](../docs/specs/pachinko.allium) specifies, written in Gleam
under `src/`. Its behaviour comes from the specification, together with
the four rulings the maintainer made on the Python double. The Rust implementation was
never its source: the Rust's tests, and the doctest that owns S005, were opened only to port the
catalog's scenarios. It serves two purposes. It is a second reading of the specification,
and it is the first of the repository's doubles built the way the specification's own
words describe an implementation: a single owner that applies a single serialized
stream, as the service's queue-owning task does
([decision 0019](../docs/decisions/0019-service-runtime-tokio-and-http.md)). Keep the
specification open beside this page; no Gleam is assumed.

Each choice the double had to make, where the specification says nothing, says it
ambiguously, or a ported scenario checks more than it says, is a `G`-note in
[`SPEC-NOTES.md`](SPEC-NOTES.md), set beside the Python double's `N`-note it agrees with
or departs from. The page refers to those notes by their ids.

Nothing here is an oracle for the Rust or for the Python double, and neither `just check`
nor CI runs it.

## How to run it

The first time in a worktree, and whenever a pin moves, fetch the toolchain and the
packages. This step needs the network:

```sh
just gleam-double-deps
```

That installs the pixi environment named `gleam`, holding `gleam` and `erlang` at the
versions `pixi.lock` pins, and fetches the hex packages `manifest.toml` pins into
`build/`, a directory Git ignores. From then on the double runs without the network:

```sh
just gleam-double
```

The recipe refuses to start on a missing or mismatched pin, then checks the layout with
`gleam format`, builds with warnings treated as errors, and runs the tests. Their entry
point, `main` in `test/pachinko_double_test.gleam`, runs the registered tests one after
another, each in a monitored process with a timeout, and then `test/support/coverage.gleam`,
before it prints the number of tests run and passed. The run fails on a failed test; on a
runner that would take a test that crashed or never returned for a pass, which the runner
checks on two such tests of its own, outside the registry; on a catalog scenario that no
passing test is tagged with; on a rule that is no table's row or whose row never fired;
and on an invariant missing from the checker or lacking a positive control. The four
sessions of the walkthrough below are tests in `test/guide_test.gleam`, and the same file
holds the check that this page still matches them.

## Where to look

| A reader looking for | Opens |
| --- | --- |
| The interface: starting the owner, the triggers, the clock and the two reads | `src/pachinko_double.gleam` |
| What a caller names when it sends a trigger or reads a reply | `config.gleam`, `time.gleam`, `model.gleam`, `refusals.gleam`, `failure.gleam` and `answers.gleam`, under `src/pachinko_double/` |
| The snapshot and the invariant checker | `inspection.gleam` and `invariants.gleam`, under `src/pachinko_double/` |
| The engine: the rule-table machinery and the apply step | `rules.gleam`, `domain.gleam` and `core.gleam`, under `src/pachinko_double/internal/` |
| The specification's rules | `identity.gleam`, `publish.gleam`, `pull.gleam`, `settle.gleam`, `lease_end.gleam`, `expiry.gleam`, `decay.gleam` and `consuming.gleam`, under `src/pachinko_double/internal/` |
| What is `pub` only for the tests | The table below this one |
| The tests | `test/*_test.gleam`, run from `test/pachinko_double_test.gleam` |
| Test support: the fakes, the runner and the coverage check | `test/support/harness.gleam`, `test/support/runner.gleam` and `test/support/coverage.gleam` |

### What is pub only for the tests

These functions are `pub` so that the tests can reach them, and no other module under
`src/` names them. Each sits under `internal/` or is marked `@internal`, and
`test/interface_test.gleam` holds this table against the source in both directions.

| Name | What the tests use it for |
| --- | --- |
| `core.run` | Running one table, a test's own included, as a whole outcome |
| `core.run_at` | The same with `now` already in hand, as a clock trigger's outcome runs |
| `core.run_due` | Running outcomes a test made by hand, one after another, as a tick does |
| `core.registry` | The coverage check reads the rules from it |
| `rules.dispatch` | Asking which row's guard holds, without running its effect |
| `invariants.registry` | The coverage check, and the positive controls, read the checker's entries |

## The specification's parts and where they live

| The specification | The double |
| --- | --- |
| An entity | A custom type in `model.gleam`, whose fields carry the specification's names |
| `config` | `Settings` in `config.gleam`, the five plain values, and `Config`, which `config.new` makes from settings that hold the config invariants |
| A rule | A row `Row(name, guard, effect)` of its trigger's table, named as the rule |
| The rules that share a trigger | One `Table(trigger, rows, otherwise)`, listed in `core.registry()` |
| An invariant | An entry `Invariant(name, holds)` of `invariants.registry()`, under the invariant's name |
| The state a rule reads and changes | `State` in `model.gleam`: the opaque `Config`, the `Queue`, whose counter is `state.queue.next_position`, and the entities' lists; `domain.gleam` binds the rules' types to it |
| The state the invariants read | `Snapshot` in `inspection.gleam`, which `inspection.snapshot` makes from the `State` an outcome left: complete and immutable, with the settings as `Settings` and the derived sets in `inspection.views` |
| A trigger a surface provides | A variant of `core.Trigger`, sent to the owner as one message by a function of `pachinko_double.gleam` |

The guard of a row holds what the rule `requires`. It takes the state and the trigger's
input and returns a `Bool`, and since nothing passes it a clock it cannot read the time.
The effect holds what the rule `ensures`, as a `Plan`: either `Finish` with the new state,
the events to emit and the answer, or a request for the time, a lease id or a position,
which the apply step fulfils (G05). `rules.gleam` is the machinery and knows no domain;
`domain.gleam` binds it to this domain's state, events, answers and refusals.

There is one apply step, `core.apply`. A trigger's table runs first. Each event that
outcome emits is then run through the table named after it: `Handed` after a pull,
`Freed` when a lease ends, `PocketTouched` after a publish into an existing pocket, and
`LeaseLapsed` from a lapse. Only when the chain is finished does the invariant checker run,
once, over the state it left. `rules.dispatch` tests every guard in a table rather than
stopping at the first that holds, so two holding at once ends the outcome with
`GuardOverlap`, and that would contradict the specification's statement that those guards
are disjoint. A table in which no guard holds answers in one of three ways:

| Tables | When no guard holds |
| --- | --- |
| `PublisherJoins`, `ConsumerJoins`, `Publish`, `Pull`, `Ack`, `Nack`, `Renew`, `Release` | `Refuses`: a refusal, one of `refusals.Refusal`, and nothing changes |
| `PocketTouched`, `LeaseLapses`, `LeaseLapsed`, `SupersededRecordDecays`, `DeadLetterDecays` | `NothingHappens` |
| `Handed`, `Freed` | `Defective`: some guard always holds, so this is a defect in the double |

A publish by a publisher id that never joined, and any action by a consumer id that never
joined, is refused by `core.apply` before a table is consulted, with `UnknownPublisher`
or `UnknownConsumer` (G12). No outcome panics: every way an outcome can fail is a
`failure.Failure`, one of `Refused`, `GuardOverlap`, `Defect` and `BrokenInvariant`, and the
owner answers with it. A call that gets no answer is another matter, under "Its mailbox"
below.

### Entities

| Entity | Type | File |
| --- | --- | --- |
| `Queue` | `Queue`, whose one field, `next_position`, only the apply step moves | `src/pachinko_double/model.gleam` |
| `Publisher` | `Publisher` | `src/pachinko_double/model.gleam` |
| `Consumer` | `Consumer` | `src/pachinko_double/model.gleam` |
| `Pocket` | `Pocket`, with `PocketStatus` | `src/pachinko_double/model.gleam` |
| `Record` | `Record`, with `RecordStatus` and `Mark` | `src/pachinko_double/model.gleam` |
| `SupersededRecord` | `SupersededRecord`, with `Reason` | `src/pachinko_double/model.gleam` |
| `DeadLetter` | `DeadLetter` | `src/pachinko_double/model.gleam` |
| `Lease` | `Lease`, with `LeaseStatus` | `src/pachinko_double/model.gleam` |

`model.State` is the whole state as one value. Where the specification has one entity
point at another, the double stores the other's key, so `Record.pocket` holds a pocket's
key and `Lease.consumer` a consumer's id (G09). The one field `Lease` adds, `id`, exists
because every action under a lease has to say which lease it means (G10). Fields the
specification declares `when status = held` exist only on the `RecordHeld` variant, and
`ended_at` only on the two variants of `LeaseStatus` for an ended lease (G14). Sets the
specification derives, `held_records`, `ending_records`, `active_leases` and the rest, are
never stored: `inspection.views` computes them from a snapshot, a `PocketView` per pocket
and a `ConsumerView` per consumer, under the specification's names. For `config`,
`config.defaults()` returns the five defaults as `Settings`, and `config.default()` the
`Config` built from them.

### Rules

The trigger column copies the rule's `when:`, which names an event, or for the three clock
rules the `Entity.field` whose time is awaited. The table column is the name the trigger's
table is registered under in `core.registry()`. The specification gives no name to a clock
trigger, so the double names each of its three clock tables, `LeaseLapses`,
`SupersededRecordDecays` and `DeadLetterDecays`, after the single rule it holds (G56).

| Rule | Trigger | Table | Function | Module |
| --- | --- | --- | --- | --- |
| `PublisherJoins` | `PublisherJoins` | `PublisherJoins` | `publisher_joins()` | `src/pachinko_double/internal/identity.gleam` |
| `ConsumerJoins` | `ConsumerJoins` | `ConsumerJoins` | `consumer_joins()` | `src/pachinko_double/internal/identity.gleam` |
| `PublishIntoNewPocket` | `Publish` | `Publish` | `publish()` | `src/pachinko_double/internal/publish.gleam` |
| `PublishNewKey` | `Publish` | `Publish` | `publish()` | `src/pachinko_double/internal/publish.gleam` |
| `Supersede` | `Publish` | `Publish` | `publish()` | `src/pachinko_double/internal/publish.gleam` |
| `DeferBehindHeldVersion` | `Publish` | `Publish` | `publish()` | `src/pachinko_double/internal/publish.gleam` |
| `DropStalePublish` | `Publish` | `Publish` | `publish()` | `src/pachinko_double/internal/publish.gleam` |
| `ReviveIdlePocket` | `PocketTouched` | `PocketTouched` | `pocket_touched()` | `src/pachinko_double/internal/publish.gleam` |
| `ReviveParkedPocket` | `PocketTouched` | `PocketTouched` | `pocket_touched()` | `src/pachinko_double/internal/publish.gleam` |
| `PullWithinLease` | `Pull` | `Pull` | `pull()` | `src/pachinko_double/internal/pull.gleam` |
| `PullClaimsFrontPocket` | `Pull` | `Pull` | `pull()` | `src/pachinko_double/internal/pull.gleam` |
| `HoldRecord` | `Handed` | `Handed` | `handed()` | `src/pachinko_double/internal/pull.gleam` |
| `PullFindsNoPocket` | `Pull` | `Pull` | `pull()` | `src/pachinko_double/internal/pull.gleam` |
| `AckMarksRecord` | `Ack` | `Ack` | `ack()` | `src/pachinko_double/internal/settle.gleam` |
| `AckRepeats` | `Ack` | `Ack` | `ack()` | `src/pachinko_double/internal/settle.gleam` |
| `NackMarksRecord` | `Nack` | `Nack` | `nack()` | `src/pachinko_double/internal/settle.gleam` |
| `NackRepeats` | `Nack` | `Nack` | `nack()` | `src/pachinko_double/internal/settle.gleam` |
| `Renew` | `Renew` | `Renew` | `renew()` | `src/pachinko_double/internal/settle.gleam` |
| `ReleaseRequeuesPocket` | `Release` | `Release` | `release()` | `src/pachinko_double/internal/settle.gleam` |
| `ReleaseIdlesEmptyPocket` | `Release` | `Release` | `release()` | `src/pachinko_double/internal/settle.gleam` |
| `LeaseLapses` | `Lease.expires_at` | `LeaseLapses` | `lease_lapses()` | `src/pachinko_double/internal/expiry.gleam` |
| `LeaseExpiryParksPocket` | `LeaseLapsed` | `LeaseLapsed` | `lease_lapsed()` | `src/pachinko_double/internal/expiry.gleam` |
| `LeaseExpiryKeepsPocketAtFront` | `LeaseLapsed` | `LeaseLapsed` | `lease_lapsed()` | `src/pachinko_double/internal/expiry.gleam` |
| `LeaseExpiryIdlesEmptyPocket` | `LeaseLapsed` | `LeaseLapsed` | `lease_lapsed()` | `src/pachinko_double/internal/expiry.gleam` |
| `RequeueUnsettledRecord` | `Freed` | `Freed` | `freed()` | `src/pachinko_double/internal/lease_end.gleam` |
| `SupersedeUnsettledHeldVersion` | `Freed` | `Freed` | `freed()` | `src/pachinko_double/internal/lease_end.gleam` |
| `ConsumeAckedRecord` | `Freed` | `Freed` | `freed()` | `src/pachinko_double/internal/lease_end.gleam` |
| `ConsumeAckedVersionQueueDeferred` | `Freed` | `Freed` | `freed()` | `src/pachinko_double/internal/lease_end.gleam` |
| `DeadLetterNackedRecord` | `Freed` | `Freed` | `freed()` | `src/pachinko_double/internal/lease_end.gleam` |
| `DeadLetterNackedVersionQueueDeferred` | `Freed` | `Freed` | `freed()` | `src/pachinko_double/internal/lease_end.gleam` |
| `SupersededRecordDecays` | `SupersededRecord.expires_at` | `SupersededRecordDecays` | `superseded_record_decays()` | `src/pachinko_double/internal/decay.gleam` |
| `DeadLetterDecays` | `DeadLetter.expires_at` | `DeadLetterDecays` | `dead_letter_decays()` | `src/pachinko_double/internal/decay.gleam` |

The function column names the public function of the module that builds the table. Within
the module, a row is usually a private function whose name is the rule's in snake case:
`supersede()` and `defer_behind_held_version()` in `publish.gleam`, for instance. The
exceptions are the rows written inline in their table, for `PublisherJoins`,
`ConsumerJoins`, `Renew` and both decays; `lease_lapses_rule()`, for `LeaseLapses`; and
the private `revive`, which builds both revive rows and is read below. All five rules that
end a lease, by release or by expiry, call `lease_end.end`, which ends the lease and
returns one `Freed` per held record, in order of position, for the six `Freed` rules.

### Invariants

`invariants.check(snapshot)` evaluates every invariant and returns the names of those that
fail. An invariant's checker entry is its `Invariant(name, holds)` in
`invariants.registry()`, where `holds` is a private function with the invariant's name in
snake case and the specification's expression as a comment above it. The declared-in
column says whether the specification states the invariant inside an entity, as it does
fifteen times, or at the top level.

| Invariant | Declared in | Checker entry | File |
| --- | --- | --- | --- |
| `LeasedHasOneActiveLease` | `Pocket` | `leased_has_one_active_lease` | `src/pachinko_double/invariants.gleam` |
| `UnleasedHasNoActiveLease` | `Pocket` | `unleased_has_no_active_lease` | `src/pachinko_double/invariants.gleam` |
| `IdleHasNoRecords` | `Pocket` | `idle_has_no_records` | `src/pachinko_double/invariants.gleam` |
| `ExpiriesWithinLimit` | `Pocket` | `expiries_within_limit` | `src/pachinko_double/invariants.gleam` |
| `HeldVersionNotNewer` | `Record` | `held_version_not_newer` | `src/pachinko_double/invariants.gleam` |
| `NackedHasItsTime` | `Record` | `nacked_has_its_time` | `src/pachinko_double/invariants.gleam` |
| `OnlyNackedHasNackTime` | `Record` | `only_nacked_has_nack_time` | `src/pachinko_double/invariants.gleam` |
| `DeferredHasArrivalPosition` | `Record` | `deferred_has_arrival_position` | `src/pachinko_double/invariants.gleam` |
| `OnlyDeferredHasArrivalPosition` | `Record` | `only_deferred_has_arrival_position` | `src/pachinko_double/invariants.gleam` |
| `DeferredArrivedAfterHeld` | `Record` | `deferred_arrived_after_held` | `src/pachinko_double/invariants.gleam` |
| `HeldCopiesAgree` | `Record` | `held_copies_agree` | `src/pachinko_double/invariants.gleam` |
| `ShedBeforeExpiry` | `SupersededRecord` | `shed_before_expiry` | `src/pachinko_double/invariants.gleam` |
| `NackedBeforeExpiry` | `DeadLetter` | `nacked_before_expiry` | `src/pachinko_double/invariants.gleam` |
| `GrantedBeforeExpiry` | `Lease` | `granted_before_expiry` | `src/pachinko_double/invariants.gleam` |
| `EndedAfterGrant` | `Lease` | `ended_after_grant` | `src/pachinko_double/invariants.gleam` |
| `ExpiryLimitPositive` | top level | `expiry_limit_positive` | `src/pachinko_double/invariants.gleam` |
| `PullBatchPositive` | top level | `pull_batch_positive` | `src/pachinko_double/invariants.gleam` |
| `LeaseTtlNonNegative` | top level | `lease_ttl_non_negative` | `src/pachinko_double/invariants.gleam` |
| `SupersededTtlNonNegative` | top level | `superseded_ttl_non_negative` | `src/pachinko_double/invariants.gleam` |
| `DeadLetterTtlNonNegative` | top level | `dead_letter_ttl_non_negative` | `src/pachinko_double/invariants.gleam` |
| `UniquePublisherId` | top level | `unique_publisher_id` | `src/pachinko_double/invariants.gleam` |
| `UniqueConsumerId` | top level | `unique_consumer_id` | `src/pachinko_double/invariants.gleam` |
| `OnePocketPerKey` | top level | `one_pocket_per_key` | `src/pachinko_double/invariants.gleam` |
| `OneRecordPerKeyPerPocket` | top level | `one_record_per_key_per_pocket` | `src/pachinko_double/invariants.gleam` |
| `OneActiveLeasePerPocket` | top level | `one_active_lease_per_pocket` | `src/pachinko_double/invariants.gleam` |
| `OneActiveLeasePerConsumer` | top level | `one_active_lease_per_consumer` | `src/pachinko_double/invariants.gleam` |
| `OutstandingOnlyWhileLeased` | top level | `outstanding_only_while_leased` | `src/pachinko_double/invariants.gleam` |
| `MarkedPocketHasNoExpiries` | top level | `marked_pocket_has_no_expiries` | `src/pachinko_double/invariants.gleam` |
| `UniquePocketPosition` | top level | `unique_pocket_position` | `src/pachinko_double/invariants.gleam` |
| `UniqueRecordPosition` | top level | `unique_record_position` | `src/pachinko_double/invariants.gleam` |
| `PocketPositionsBelowNext` | top level | `pocket_positions_below_next` | `src/pachinko_double/invariants.gleam` |
| `RecordPositionsBelowNext` | top level | `record_positions_below_next` | `src/pachinko_double/invariants.gleam` |
| `ArrivalPositionUnoccupied` | top level | `arrival_position_unoccupied` | `src/pachinko_double/invariants.gleam` |
| `UniqueArrivalPosition` | top level | `unique_arrival_position` | `src/pachinko_double/invariants.gleam` |
| `ArrivalPositionsBelowNext` | top level | `arrival_positions_below_next` | `src/pachinko_double/invariants.gleam` |

`config.new` enforces the five config invariants as well, refusing settings that break one
with `ConfigRefused`, so an owner cannot start with them (G02). For each invariant
`test/invariants_test.gleam` holds a test tagged `InvariantControl` and the invariant's
name, which builds a snapshot that breaks it and shows that `check` names it.

## Time, identity and inspection

Several things the specification leaves to the clock or to whoever implements it are, in
the double, explicit inputs or explicit messages, so that a run repeats exactly.

- **The owner.** One `gleam_otp` actor, started by `pachinko_double.start`, holds the queue's state,
  its config, the clock function and the lease-id function. No other process holds any
  of the queue's state: not one per pocket, lease or consumer, not the test runner's, not
  the fakes'. Nothing runs on a timer. `pachinko_double.gleam`, the root module, is the only
  module under `src/` that imports `gleam/erlang` or `gleam/otp`; the rest of `src/` is pure.
- **Its mailbox, the one serialized stream.** The owner's `Message` type has four
  variants: `Act`, which wraps a `core.Trigger`, the type with a variant for each trigger
  the two surfaces provide; `Tick`; and the reads `Inspect` and `LeaseView`. Messages are
  handled strictly one at a time. On `Act` the owner runs `core.apply`, supplies whatever
  the resulting step asks for through `rules.drive`, and replies. It keeps the new state
  only on success: a refusal, a guard overlap, a defect or a broken invariant leaves the
  state as it was. This mailbox is the serialized stream that the specification's Lease
  expiry comment and
  [decision 0019](../docs/decisions/0019-service-runtime-tokio-and-http.md) ask for.
  Callers use one function of `pachinko_double.gleam` per trigger,
  `pachinko_double.publish`, `pachinko_double.ack` and so on, each waiting for the reply
  for at most `pachinko_double.call_timeout`, 1,000 ms. A caller
  that gets no reply in that time, or whose owner has died, crashes; it receives no
  `failure.Failure`. The trigger it sent may still be applied after it has crashed: while
  the owner is held up, by a slow injected clock for instance, a publish
  whose caller has timed out stays in the mailbox and is applied once the owner reaches
  it. In a test the crash fails the test. Success is `Outcome(fired, answer)`, the
  names of the rules that fired in firing order and the answer. Only a pull answers
  something, a `Batch` or `NoPocketAvailable`; the specification gives the other triggers
  no answer, so the tests read their effect from a snapshot (G15).
- **The injected clock.** `pachinko_double.start` takes `now: fn() -> Timestamp`, counted in whole
  milliseconds (G01). The clock is read only when a row's effect asks for `now`, and at
  most once in an outcome; a refusal never reads it, because only an effect can ask. The
  double contains no real clock. Tests pass the fake from `test/support/harness.gleam`, which
  `harness.set_time` moves and which counts how often it is read.
- **The lease-id source.** The specification's `Lease` has no identity, yet ack, nack,
  renew and release each name a lease, so the double gives each lease an id (G10). The
  owner's `lease_id: fn() -> String` is called only when `PullClaimsFrontPocket` grants a
  lease. The fake in `test/support/harness.gleam` counts its calls and hands out `lease-1`, then
  `lease-2`, and onward.
- **`Tick`.** Time passes for the double only when the owner receives `Tick`, sent by
  `pachinko_double.tick`; until then no lease expires and no shed entry decays. On a tick the owner
  reads the clock once and calls `core.tick(state, now)`. First come the lapses: every
  active lease with `expires_at <= now`, earliest `expires_at` first and ties in creation
  order (G57), each one outcome through `LeaseLapses` and `LeaseLapsed`. Then the decays:
  every superseded record and then every dead letter with `expires_at <= now`, each one
  outcome. The comparison with `now` is how the tick chooses what is due, not a guard, and
  the `LeaseLapsed` event carries the tick's `now` for the expiry rules' guards to compare
  with (G60). A tick replies with a `TickReport`: the rules fired, the leases lapsed and
  the entries decayed. Should an outcome fail, the tick stops at it, keeps every earlier
  outcome, and replies `PartialTick` (G59). An action processed after a lease's
  `expires_at` but before the tick that lapses it therefore succeeds, as branch A of
  decision 0019 says it must.
- **The inspection seam.** `pachinko_double.inspect` sends `Inspect` and receives an
  `inspection.Snapshot` of the whole state: the settings, `next_position`, and the lists of
  publishers, consumers, pockets, records, leases, superseded records and dead letters,
  each in creation order so that snapshots compare with `==` (G06, G19). Derived sets are
  not part of it; `inspection.views(snapshot)` derives them once, and the checker reads
  nothing but a snapshot and its views. `pachinko_double.lease_view` sends `LeaseView` and receives
  `consuming.lease_view`'s answer, what the `Consuming` surface shows one consumer. An
  ended lease remains in the snapshot, under the maintainer's ruling on the Python
  double's N24. The lists' order is the double's choice, not the specification's, which is
  why the walkthrough looks records up by key.

## Reading a row: the Gleam you need

These excerpts are copied verbatim from the files they name, and the run fails the moment
one stops matching.

### A custom type

A custom type is a list of variants, and each variant is a constructor that can carry
labelled fields. Here a record's status is `RecordQueued`, with no fields, or
`RecordHeld`, with exactly the five fields the specification declares
`when status = held`; a queued record therefore has nowhere to keep them.
`Option(Timestamp)` stands for the specification's `Timestamp?` and is `None` or
`Some(value)`. A line starting `///` is a documentation comment.

```gleam src/pachinko_double/model.gleam
/// A record's status. The five fields the specification declares `when
/// status = held` are fields of the held variant alone, so a queued record
/// cannot carry one.
pub type RecordStatus {
  RecordQueued
  RecordHeld(
    held_rank: Int,
    held_payload: BitArray,
    mark: Mark,
    nacked_at: Option(Timestamp),
    deferred_position: Option(Int),
  )
}
```

Calling a constructor with its labels makes a value, `Pocket(key: "pk", ...)` for
instance. `State(..state, publishers:)` is `state` with that one field replaced, and a
bare label such as `publishers:` stands for `publishers: publishers`. Payloads are
`BitArray`s; `<<"v1":utf8>>` is the UTF-8 bytes of `v1` (G07).

### A row

`fn(arguments) { body }` is an anonymous function, and a function's value is its last
expression. A `Table` takes the trigger's name, its rows and what to do when no guard
holds. Each `Row` takes the rule's name, then its guard, then its effect. In the table
below, for `PublisherJoins`, the guard asks that the id is not taken, and the effect adds
the publisher and ends with `Finish`, emitting no event and answering `None`. If the guard
fails, `Refuses` names the refusal.

```gleam src/pachinko_double/internal/identity.gleam
/// `PublisherJoins(publisher_id)`. A join with an id already taken is
/// refused (spec note G03).
pub fn publisher_joins() -> Table(String) {
  Table(
    "PublisherJoins",
    [
      Row(
        "PublisherJoins",
        fn(state: State, publisher_id) {
          !list.contains(state.publishers, Publisher(id: publisher_id))
        },
        fn(state, publisher_id) {
          let publishers =
            list.append(state.publishers, [Publisher(id: publisher_id)])
          Finish(State(..state, publishers:), [], None)
        },
      ),
    ],
    Refuses(fn(_state, publisher_id) { PublisherAlreadyJoined(publisher_id) }),
  )
}
```

### A `Result` and a `case`

A lookup that may fail returns a `Result`: `Ok(value)` when it finds something, `Error`
otherwise. A `case` tries its patterns in order and evaluates the branch of the first one
that matches. `revive` builds either revive row, and its guard is a `case` over the
`Result` of finding the pocket:

```gleam src/pachinko_double/internal/publish.gleam
/// The two revive rules, which differ in the status they require:
///
/// ```text
/// requires: exists pocket
/// requires: pocket.status = idle        -- or parked
/// let position = queue.next_position
/// ensures: pocket.status = queued
/// ensures: pocket.position = position
/// ensures: pocket.consecutive_expiries = 0
/// ensures: queue.next_position = position + 1
/// ```
fn revive(name: String, from: PocketStatus) -> Row(String) {
  Row(
    name,
    fn(state: State, pocket_key: String) {
      case model.pocket(state.pockets, pocket_key) {
        Error(Nil) -> False
        Ok(pocket) -> pocket.status == from
      }
    },
    fn(state, pocket_key) {
      case model.pocket(state.pockets, pocket_key) {
        Error(Nil) -> Impossible("no pocket " <> pocket_key)
        Ok(pocket) -> {
          use position <- Position
          let queued =
            Pocket(
              ..pocket,
              status: PocketQueued,
              position:,
              consecutive_expiries: 0,
            )
          Finish(
            State(..state, pockets: model.replace_pocket(state.pockets, queued)),
            [],
            None,
          )
        }
      }
    },
  )
}
```

`Impossible` ends an effect that cannot find what its guard saw, and the apply step reports
it as a `Defect`. `Position`, and the `use` in front of it, come next.

### `use`

`use x <- f(arguments)` passes everything after it in the block to `f`, as a function that
takes `x`. In `revive`, `use position <- Position` wraps the rest of the effect in a
function held by `Position`: the effect asks for a position, and the apply step calls that
function with the queue's counter and then advances the counter. `Now` and `LeaseId` ask
for the time and for a lease id the same way. With `result.try`, as in the Publish
module's lookup of a record below, the rest runs only on an `Ok`, and an `Error` is
handed back unchanged:

```gleam src/pachinko_double/internal/publish.gleam
/// `Record{pocket: pocket, key: record_key}`, where `pocket` is
/// `Pocket{key: pocket_key}`: no pocket, no record.
fn record(state: State, publication: Publication) -> Result(Record, Nil) {
  use pocket <- result.try(pocket(state, publication))
  model.record(state.records, pocket: pocket.key, key: publication.record_key)
}
```

The tests use two more forms. `assert left == right` fails the test if the sides differ,
and `let assert Ok(x) = expression` fails it if the expression is anything but `Ok`. No
code under `src/` uses either, since there every failure is a value.

## Walkthrough: Scenarios A to D

The [scenario catalog](../docs/reference/testing.md#scenario-catalog) names four cases of
the reconciliation at a lease's end that
[decision 0020](../docs/decisions/0020-held-set-until-lease-ends.md) adopted: Scenarios A
to D, rows S078 to S081. All four begin alike: under its lease, `c1` holds `k` at rank
1 with payload `v1`, and a higher rank of `k` is published and waits behind it. How the
lease's end treats `k` then turns on whether `c1` acked it, nacked it or neither. The
sessions here end Scenarios A and C with a release and B and D with a lapse.
`test/lease_endings_test.gleam` holds the catalog's tagged ports, which run each scenario
to both endings from the Rust's fixture.

Each block in this part is lifted verbatim from `test/guide_test.gleam`, where it is one
step of the scenario's registered test, tagged with the scenario's catalog id. A step
sends the owner its triggers and then asserts the state they left, so everything printed
here is something a test checks. Positions start at 0 (G04) and lease ids are the fake
source's. The specification fixes neither number: it fixes only that positions are unique
and handed out in increasing order.

### The setting

`setting` starts the owner over a configuration with short TTLs and a fake clock reading
0, and publisher `p` and consumers `c1` and `c2` join:

```gleam
  let assert Ok(config) =
    config.new(
      Settings(
        ..config.defaults(),
        lease_ttl: 1000,
        superseded_ttl: 5000,
        dead_letter_ttl: 7000,
      ),
    )
  let double = harness.start(config)
  assert harness.publisher_joins(double, "p") == fired(["PublisherJoins"])
  assert harness.consumer_joins(double, "c1") == fired(["ConsumerJoins"])
  assert harness.consumer_joins(double, "c2") == fired(["ConsumerJoins"])
```

### What a step shows

After a step, `show` reads one snapshot through the owner and returns a `Shown`: the
position counter, pocket `pk`, those records of `pk` whose keys it is given, found by key,
all leases, and both shed queues. The owner's reply to a trigger that succeeded lists the
rules it fired, and `fired` builds that reply for a trigger with no answer.

```gleam
type Shown {
  Shown(
    next_position: Int,
    pocket: Pocket,
    records: List(Record),
    leases: List(Lease),
    superseded: List(SupersededRecord),
    dead_letters: List(DeadLetter),
  )
}
```

```gleam
fn show(double: Double, keys: List(String)) -> Shown {
  let snapshot = harness.inspect(double)
  let assert Ok(pocket) = model.pocket(snapshot.pockets, "pk")
  Shown(
    next_position: snapshot.next_position,
    pocket:,
    records: list.map(keys, fn(key) {
      let assert Ok(record) = model.record(snapshot.records, pocket: "pk", key:)
      record
    }),
    leases: snapshot.leases,
    superseded: snapshot.superseded,
    dead_letters: snapshot.dead_letters,
  )
}
```

```gleam
fn fired(rules: List(String)) -> Reply {
  Ok(Outcome(rules, None))
}
```

```gleam
fn c2_pulls(double: Double) -> List(String) {
  harness.keys(harness.batch(double, "c2", 10).records)
}
```

### The common start

The four sessions share `v3_deferred_behind_v1`. First, `k` at rank 1 goes into `pk`, a
pocket that does not exist yet, so `PublishIntoNewPocket` creates it: the pocket takes
position 0 and the record position 1.

```gleam
  let double = setting()
  assert harness.publish(double, "pk", "k", 1, "v1")
    == fired(["PublishIntoNewPocket"])
  assert show(double, ["k"])
    == Shown(
      next_position: 2,
      pocket: Pocket(
        key: "pk",
        status: PocketQueued,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 1,
          payload: <<"v1":utf8>>,
          position: 1,
          status: RecordQueued,
        ),
      ],
      leases: [],
      superseded: [],
      dead_letters: [],
    )
```

Then `early` goes into the same pocket, a new key there, and `PublishNewKey` gives it
position 2:

```gleam
  assert harness.publish(double, "pk", "early", 1, "e")
    == fired(["PublishNewKey"])
  assert show(double, ["k", "early"])
    == Shown(
      next_position: 3,
      pocket: Pocket(
        key: "pk",
        status: PocketQueued,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 1,
          payload: <<"v1":utf8>>,
          position: 1,
          status: RecordQueued,
        ),
        Record(
          pocket: "pk",
          key: "early",
          rank: 1,
          payload: <<"e":utf8>>,
          position: 2,
          status: RecordQueued,
        ),
      ],
      leases: [],
      superseded: [],
      dead_letters: [],
    )
```

`c1` pulls with a limit of one. It holds no lease, so `PullClaimsFrontPocket` grants it
one on the queued pocket with the smallest position, `pk`, expiring `lease_ttl` after now.
The batch is the oldest record, `k`, and the `Handed` for it reaches `HoldRecord`, which
marks the record held and unsettled with `held_rank` and `held_payload` copied from it:

```gleam
  assert harness.pull(double, "c1", 1)
    == Ok(Outcome(
      ["PullClaimsFrontPocket", "HoldRecord"],
      Some(
        Batch(consumer: "c1", lease: "lease-1", pocket: "pk", records: [
          BatchRecord(key: "k", rank: 1, payload: <<"v1":utf8>>),
        ]),
      ),
    ))
  assert show(double, ["k"])
    == Shown(
      next_position: 3,
      pocket: Pocket(
        key: "pk",
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 1,
          payload: <<"v1":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: None,
          ),
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1000,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )
```

At 10 ms `k` is published at rank 3, above its stored rank. Because the record is held
and nothing waits behind it yet, `DeferBehindHeldVersion` is the rule that applies. The
record's `rank` and `payload` become `v3`'s, `held_rank` and `held_payload` keep `v1`, and
`deferred_position` takes the counter's value, 3, as `v3`'s arrival position. The
`PocketTouched` it emits finds the pocket leased, so no revive rule fires.

```gleam
  harness.set_time(double.clock, 10)
  assert harness.publish(double, "pk", "k", 3, "v3")
    == fired(["DeferBehindHeldVersion"])
  assert show(double, ["k"])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: "pk",
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: Some(3),
          ),
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1000,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )
```

### Scenario A: an acked version is consumed at the lease's end, and the deferred one queued at its arrival position

At 20 ms `c1` acks `k`. `AckMarksRecord` sets the mark to acked and moves the lease's
`expires_at` to 20 ms plus `lease_ttl`. The record stays where it is:

```gleam
  let double = v3_deferred_behind_v1()
  harness.set_time(double.clock, 20)
  assert harness.ack(double, "c1", "lease-1", "k") == fired(["AckMarksRecord"])
  assert show(double, ["k"])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: "pk",
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Acked,
            nacked_at: None,
            deferred_position: Some(3),
          ),
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )
```

At 50 ms `c1` releases its lease. Two records outlast it, `early` and `k`, whose newer
version is waiting, so `ReleaseRequeuesPocket` applies: the lease ends `released` and the
pocket is queued again at a fresh position, 4. The rule sends `Freed` for `k`, and
`ConsumeAckedVersionQueueDeferred` answers it. The acked `v1` leaves without going to
either shed queue, and `v3` is queued at 3, the position it arrived at, after `early` at 2.
`c2`'s pull shows that order:

```gleam
  harness.set_time(double.clock, 50)
  assert harness.release(double, "c1", "lease-1")
    == fired(["ReleaseRequeuesPocket", "ConsumeAckedVersionQueueDeferred"])
  assert show(double, ["k"])
    == Shown(
      next_position: 5,
      pocket: Pocket(
        key: "pk",
        status: PocketQueued,
        position: 4,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 3,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseReleased(ended_at: 50),
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )
  assert c2_pulls(double) == ["early", "k"]
```

### Scenario B: a nacked version is dead-lettered at the lease's end, and the deferred one queued at its arrival position

At 20 ms `c1` nacks `k`. `NackMarksRecord` sets the mark to nacked, records 20 ms as
`nacked_at` and extends the lease. Nothing is dead-lettered while the lease lasts:

```gleam
  let double = v3_deferred_behind_v1()
  harness.set_time(double.clock, 20)
  assert harness.nack(double, "c1", "lease-1", "k")
    == fired(["NackMarksRecord"])
  assert show(double, ["k"])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: "pk",
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Nacked,
            nacked_at: Some(20),
            deferred_position: Some(3),
          ),
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )
```

`c1` sends nothing more, the clock is set to the lease's `expires_at`, and a tick runs.
`LeaseLapses` finds the lease active and due, and its `LeaseLapsed` reaches the three
expiry rules. The count this lapse brings the pocket to, 1, is below
`max_consecutive_expiries`, 3, and `early` and `k` outlast the lease, so
`LeaseExpiryKeepsPocketAtFront` holds: the lease ends `expired`,
`consecutive_expiries` becomes 1, and the pocket is queued with its old position, 0.
`DeadLetterNackedVersionQueueDeferred` answers the `Freed` for `k`. `v1` becomes a dead
letter that keeps its `nacked_at`, 20, and expires `dead_letter_ttl` after the lease ended,
and `v3` is queued at its arrival position:

```gleam
  harness.set_time(double.clock, 1020)
  let expired =
    Lease(
      id: "lease-1",
      pocket: "pk",
      consumer: "c1",
      status: LeaseExpired(ended_at: 1020),
      granted_at: 0,
      expires_at: 1020,
    )
  assert harness.tick(double)
    == Ok(
      TickReport(
        fired: [
          "LeaseLapses",
          "LeaseExpiryKeepsPocketAtFront",
          "DeadLetterNackedVersionQueueDeferred",
        ],
        lapsed: [expired],
        superseded: [],
        dead_letters: [],
      ),
    )
  assert show(double, ["k"])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: "pk",
        status: PocketQueued,
        position: 0,
        consecutive_expiries: 1,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 3,
          status: RecordQueued,
        ),
      ],
      leases: [expired],
      superseded: [],
      dead_letters: [
        DeadLetter(
          pocket: "pk",
          key: "k",
          rank: 1,
          payload: <<"v1":utf8>>,
          nacked_by: "c1",
          nacked_at: 20,
          expires_at: 8020,
        ),
      ],
    )
  assert c2_pulls(double) == ["early", "k"]
```

### Scenario C: v3 is shed when v5 arrives; v1, nacked, is dead-lettered at the end; v5 takes v3's arrival position

At 15 ms a new key, `between`, is published into `pk`; `PublishNewKey` gives it the
counter's value, 4:

```gleam
  let double = v3_deferred_behind_v1()
  harness.set_time(double.clock, 15)
  assert harness.publish(double, "pk", "between", 1, "b")
    == fired(["PublishNewKey"])
  assert show(double, ["k", "between"])
    == Shown(
      next_position: 5,
      pocket: Pocket(
        key: "pk",
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: Some(3),
          ),
        ),
        Record(
          pocket: "pk",
          key: "between",
          rank: 1,
          payload: <<"b":utf8>>,
          position: 4,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1000,
        ),
      ],
      superseded: [],
      dead_letters: [],
    )
```

At 18 ms `k` is published at rank 5. A version, `v3`, already waits behind the held one,
so this is `Supersede`, not a second deferral. Nobody holds `v3`, and it goes to the
superseded queue at once, shed at 18 ms. The record's `rank` and `payload` become `v5`'s,
`deferred_position` stays 3, which `v5` inherits from `v3`, and the held `v1` is
untouched:

```gleam
  harness.set_time(double.clock, 18)
  assert harness.publish(double, "pk", "k", 5, "v5") == fired(["Supersede"])
  let v3 =
    SupersededRecord(
      pocket: "pk",
      key: "k",
      rank: 3,
      payload: <<"v3":utf8>>,
      reason: Superseded,
      shed_at: 18,
      expires_at: 5018,
    )
  assert show(double, ["k", "between"])
    == Shown(
      next_position: 5,
      pocket: Pocket(
        key: "pk",
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 5,
          payload: <<"v5":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Unsettled,
            nacked_at: None,
            deferred_position: Some(3),
          ),
        ),
        Record(
          pocket: "pk",
          key: "between",
          rank: 1,
          payload: <<"b":utf8>>,
          position: 4,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1000,
        ),
      ],
      superseded: [v3],
      dead_letters: [],
    )
```

At 20 ms `c1` nacks `k`. The mark falls on the version `c1` was handed, `v1`:

```gleam
  harness.set_time(double.clock, 20)
  assert harness.nack(double, "c1", "lease-1", "k")
    == fired(["NackMarksRecord"])
  assert show(double, ["k", "between"])
    == Shown(
      next_position: 5,
      pocket: Pocket(
        key: "pk",
        status: PocketLeased,
        position: 0,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 5,
          payload: <<"v5":utf8>>,
          position: 1,
          status: RecordHeld(
            held_rank: 1,
            held_payload: <<"v1":utf8>>,
            mark: Nacked,
            nacked_at: Some(20),
            deferred_position: Some(3),
          ),
        ),
        Record(
          pocket: "pk",
          key: "between",
          rank: 1,
          payload: <<"b":utf8>>,
          position: 4,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseActive,
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [v3],
      dead_letters: [],
    )
```

At 50 ms `c1` releases. `DeadLetterNackedVersionQueueDeferred` makes `v1` a dead letter
that expires `dead_letter_ttl` after the release, and queues `v5` at the arrival position
it inherited, 3, so `c2` is handed it before `between`, at 4:

```gleam
  harness.set_time(double.clock, 50)
  assert harness.release(double, "c1", "lease-1")
    == fired(["ReleaseRequeuesPocket", "DeadLetterNackedVersionQueueDeferred"])
  assert show(double, ["k", "between"])
    == Shown(
      next_position: 6,
      pocket: Pocket(
        key: "pk",
        status: PocketQueued,
        position: 5,
        consecutive_expiries: 0,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 5,
          payload: <<"v5":utf8>>,
          position: 3,
          status: RecordQueued,
        ),
        Record(
          pocket: "pk",
          key: "between",
          rank: 1,
          payload: <<"b":utf8>>,
          position: 4,
          status: RecordQueued,
        ),
      ],
      leases: [
        Lease(
          id: "lease-1",
          pocket: "pk",
          consumer: "c1",
          status: LeaseReleased(ended_at: 50),
          granted_at: 0,
          expires_at: 1020,
        ),
      ],
      superseded: [v3],
      dead_letters: [
        DeadLetter(
          pocket: "pk",
          key: "k",
          rank: 1,
          payload: <<"v1":utf8>>,
          nacked_by: "c1",
          nacked_at: 20,
          expires_at: 7050,
        ),
      ],
    )
  assert c2_pulls(double) == ["early", "k", "between"]
```

### Scenario D: an unsettled held version is superseded at the lease's end, and the newer version takes its position

This time `c1` neither acks nor nacks, and the tick at the lease's `expires_at` lapses it
as in Scenario B. `SupersedeUnsettledHeldVersion` answers the `Freed` for `k`: the
unsettled `v1` goes to the superseded queue, shed at the moment the lease ended, and `v3`
is queued at `v1`'s own position, 1. Since nobody marked `k`, the key keeps its place,
and `c2` is handed it before `early`:

```gleam
  let double = v3_deferred_behind_v1()
  harness.set_time(double.clock, 1000)
  let expired =
    Lease(
      id: "lease-1",
      pocket: "pk",
      consumer: "c1",
      status: LeaseExpired(ended_at: 1000),
      granted_at: 0,
      expires_at: 1000,
    )
  assert harness.tick(double)
    == Ok(
      TickReport(
        fired: [
          "LeaseLapses",
          "LeaseExpiryKeepsPocketAtFront",
          "SupersedeUnsettledHeldVersion",
        ],
        lapsed: [expired],
        superseded: [],
        dead_letters: [],
      ),
    )
  assert show(double, ["k"])
    == Shown(
      next_position: 4,
      pocket: Pocket(
        key: "pk",
        status: PocketQueued,
        position: 0,
        consecutive_expiries: 1,
      ),
      records: [
        Record(
          pocket: "pk",
          key: "k",
          rank: 3,
          payload: <<"v3":utf8>>,
          position: 1,
          status: RecordQueued,
        ),
      ],
      leases: [expired],
      superseded: [
        SupersededRecord(
          pocket: "pk",
          key: "k",
          rank: 1,
          payload: <<"v1":utf8>>,
          reason: Superseded,
          shed_at: 1000,
          expires_at: 6000,
        ),
      ],
      dead_letters: [],
    )
  assert c2_pulls(double) == ["k", "early"]
```

The checker ran at the end of every outcome above, and the state the session ends in
still breaks no invariant:

```gleam
  assert invariants.check(harness.inspect(double)) == []
```

## Keeping this page true

Gleam has no doctests. Instead, every run of `just gleam-double` reads this page and
`test/guide_test.gleam` through `simplifile`, and `drift` fails the run on any of these:

- a block fenced as `gleam` whose lines do not appear, whole and in order, indentation
  included, in `test/guide_test.gleam`;
- a section for one of Scenarios A to D whose blocks of the walkthrough, joined by a blank
  line, are not the whole body of its test, `scenario_a` to `scenario_d`, less the last
  line, which returns the rules the test saw fire. A block repeated, moved or left out is
  a fault;
- a section for the setting or for the common start, under exactly `### The setting` or
  `### The common start`, whose blocks, joined the same way, are not the whole body of
  `setting` or of `v3_deferred_behind_v1`, less the last line, which returns the double;
- a block fenced as `gleam` and a path under `src/` whose lines do not appear the same way
  in that file;
- any other fence that names `gleam`, one inside a blockquote among them, a fence left
  open, or an empty block. As in Markdown, a fence closes on a run of its mark at least as
  long as the one that opened it;
- the setting, the common start or a scenario among A to D without a section, with two,
  or with a section holding no block of the walkthrough, or a section for a scenario this
  page does not walk.

The same file holds a control for each of those faults, run on a page and files built by
hand.
