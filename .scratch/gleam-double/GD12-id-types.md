---
id: GD12
title: "Gleam double: a type of its own for each id"
status: todo
depends_on: [GD11]
parallel_with: []
branch: build-gleam-double
estimated_size: XL
---

# GD12: Gleam double: a type of its own for each id

## Context

After GD08 to GD11 the double's interface is one place: the root module,
`gleam-double/src/pachinko_double.gleam`, and the vocabulary modules beside it. Every id
in it is still a `String`. The facts below are those of commit
`f56ece6caf0b7381e9b5671f9b4b38b378e1d75d`, which holds GD11 and the series review's
fixes.

So a call with two ids swapped compiles. `pachinko_double.renew(owner, consumer, lease)`
takes two `String`s, and `ack` and `nack` take three. A swapped publish is worse than a
refusal: `harness.publish(double, "a", "pk", 1, "v1")`, with the pocket key and the
record key swapped, succeeds, and creates pocket `a` holding record `pk`.

The specification names four of the ids and types each as a `String`:

| The specification | Declared as | Carried by the triggers as |
| --- | --- | --- |
| A publisher's id | `Publisher.id: String` | `PublisherJoins(publisher_id)`, and `publisher` in `Publish` |
| A consumer's id | `Consumer.id: String` | `ConsumerJoins(consumer_id)`, and `consumer` in `Pull`, `Ack`, `Nack`, `Renew` and `Release` |
| A pocket key | `Pocket.key: String` | `pocket_key` in `Publish` |
| A record key | `Record.key: String`, and `key` in `SupersededRecord` and `DeadLetter` | `record_key` in `Publish`, `Ack` and `Nack` |

It states no format for any of them: no length, no character set, nothing about an empty
id. Its vocabulary says only that a pocket is "named by a pocket key" and that a record
is "the one slot a record key has in a pocket". The fifth id, the lease's, is the
double's own: the specification's `Lease` declares no identity, and spec note G10 gives
each lease an `id`, a `String` drawn from the owner's injected lease-id function. A
reference to another entity holds that entity's key or id (G09), so `Record.pocket` is a
pocket key and `Lease.consumer` a consumer's id.

Two facts of the import graph bear on where new types can live. `config` imports
`refusals`, for `ConfigRefused`, and `model` imports `config`; so `refusals` cannot
import `model`. And `internal/rules.gleam`, the rule-table machinery, imports only
`failure` and `time`, yet it carries the lease id: its `LeaseId` request hands an effect
a `String`, and `rules.drive` takes `lease_id: fn() -> String`.

## Goal

Passing one id where another is expected does not compile, anywhere in the double. Each
of the five ids is a type of its own, from the root's functions through the model, the
replies and the refusals to the engine. The pin holds it: no `pub fn` of the root names
`String`, and the lines under `gleam-double/src/` that name `String` are exactly the 32
that hold no id.

This ticket owns:

| Kind | Names |
| --- | --- |
| Rules (0) | none |
| Scenarios (0) | none |

## Decisions fixed in this ticket

The maintainer fixed the first group on 2026-11-23. The builder does not reopen them.

- **The ids get types of their own, so a swapped argument no longer compiles**: the
  publisher's id, the consumer's id, the pocket key, the record key and the lease id.
  Each takes the specification's name where it has one: `PublisherId` for
  `Publisher.id`, `ConsumerId` for `Consumer.id`, `PocketKey` for `Pocket.key` and
  `RecordKey` for `Record.key`. The specification declares the lease no identity, so
  `LeaseId` follows G10's `Lease.id`. The specification states no format for any of
  them.
- **The types reach the whole interface and the model**: every trigger function and the
  two reads in the root, the model's records and leases, `Publication`, the answers and
  views, the snapshot, and the engine. One vocabulary end to end; no `String` id is left
  in a public signature.
- **Each is an open single-constructor custom type**, such as
  `pub type PocketKey { PocketKey(String) }`, in the public vocabulary: `model.gleam`,
  unless the reading gives a reason for another public module, which the ticket then
  states. No validation: callers construct them and match them. A type alias is not
  enough, because an alias is transparent.
- **No behaviour changes.** The coverage check stays strict at 93, 32 and 35.

The second group is this ticket's own design, written from a scan of the tree at the
commit Context names. The builder follows it, and reports under Departures if one of
these cannot hold. Each count's command is in "Files touched".

- **The types live in a new public module, `gleam-double/src/pachinko_double/ids.gleam`,
  which imports nothing.** `model.gleam` cannot hold them, for two reasons:
  - `refusals` names four of the ids in its constructors (`PublisherAlreadyJoined`,
    `LeaseFenced` and the rest), so it must import their module. `refusals` cannot import
    `model`, because `model` imports `config` and `config` imports `refusals`: that is a
    cycle.
  - `internal/rules.gleam` needs `LeaseId` for its request and for `drive`. Taking it
    from `model` would make the domain-free machinery import the domain's entities.

  `ids` sits below both, beside `failure`, which also imports nothing. Its text is fixed
  here, so that the builder writes no prose of their own into it. Each type is written in
  the three-line shape that `gleam format` produces, which the pin's parser reads as one
  item with one constructor; GD08's item control already shows that an alias and an
  opaque type read as items with no constructor.

  ```gleam
  //// The ids and keys a caller names. The specification declares each as a
  //// `String` and states no format, so each type here wraps any `String` and
  //// checks nothing: a caller makes one with its constructor and reads one by
  //// matching it. Each is a type of its own so that one cannot be passed where
  //// another is expected.

  /// A publisher's id: `Publisher.id`, and the `publisher_id` and `publisher`
  /// the `Publishing` surface's triggers carry.
  pub type PublisherId {
    PublisherId(String)
  }

  /// A consumer's id: `Consumer.id`, the `consumer_id` and `consumer` the
  /// `Consuming` surface's triggers carry, and a reference to a consumer.
  pub type ConsumerId {
    ConsumerId(String)
  }

  /// A pocket's key: `Pocket.key`, the `pocket_key` a publish carries, and a
  /// reference to a pocket.
  pub type PocketKey {
    PocketKey(String)
  }

  /// A record's key within its pocket: `Record.key`, the `record_key` a
  /// publish, an ack and a nack carry, and the key of a shed version.
  pub type RecordKey {
    RecordKey(String)
  }

  /// A lease's id. The specification declares none; the double gives each
  /// lease one, drawn from the owner's lease-id function (spec note G10).
  pub type LeaseId {
    LeaseId(String)
  }
  ```

- **Every field and parameter that holds an id takes its type.** No field, label or
  argument is renamed, reordered, added or removed; only the type changes. At the base,
  105 lines under `gleam-double/src/`, in 14 modules, type an id as `String`:

  | Module | Lines | What takes which type |
  | --- | --- | --- |
  | `model` | 21 | `Publisher.id`: `PublisherId`. `Consumer.id` and `DeadLetter.nacked_by`: `ConsumerId`. `Pocket.key`, and `pocket` in `Record`, `SupersededRecord`, `DeadLetter` and `Lease`: `PocketKey`. `key` in `Record`, `SupersededRecord` and `DeadLetter`: `RecordKey`. `Lease.id`: `LeaseId`; `Lease.consumer`: `ConsumerId`. The lookups follow: `pocket`, `record`, `held_under`, `lease`, `records_of`, `leases_of_pocket` and `leases_of_consumer` |
  | The root | 20 | The ten functions that take an id, `start`'s `lease_id: fn() -> LeaseId`, the private `Owner`'s `lease_id`, and the `consumer` of `Message`'s `LeaseView` |
  | `internal/settle` | 19 | The inputs `Ack`, `Nack`, `Renew` and `Release`, and the private helpers' parameters |
  | `refusals` | 10 | `publisher_id`: `PublisherId`; `consumer_id`: `ConsumerId`; `pocket_key`: `PocketKey`; `record_key`: `RecordKey`; `lease_id`: `LeaseId`. `action` and `ConfigRefused`'s `invariants` stay `String` |
  | `internal/publish` | 8 | `Publication`'s three ids; `pocket_touched` becomes a `Table(PocketKey)`, the two revive rows and `revive` `Row(PocketKey)`s, and the guard's `pocket_key` a `PocketKey` |
  | `answers` | 7 | `Batch`'s `consumer`, `lease` and `pocket`; `NoPocketAvailable`'s `consumer`; `BatchRecord.key` and `HeldView.key`: `RecordKey`; `LeaseView.pocket_key`: `PocketKey` |
  | `internal/core` | 5 | `Trigger`'s `PublisherJoins` and `ConsumerJoins`, and three private helpers' `consumer_id` |
  | `internal/domain` | 4 | The events: `PocketTouched`, `Handed`, `Freed` and `LeaseLapsed` |
  | `internal/rules` | 3 | `LeaseId(next: fn(ids.LeaseId) -> Plan(..))`, `NeedsLeaseId(next: fn(ids.LeaseId) -> Step(..))` and `drive`'s `lease_id: fn() -> ids.LeaseId` |
  | `internal/identity` | 2 | `publisher_joins` becomes a `Table(PublisherId)`, `consumer_joins` a `Table(ConsumerId)` |
  | `internal/lease_end` | 2 | `outlasts`'s `pocket_key`; `Freeing`'s `lease` and `record_key` |
  | `internal/pull` | 2 | `Pull.consumer`; `Handover`'s `pocket_key` and `record_key` |
  | `internal/consuming` | 1 | `lease_view`'s `consumer` |
  | `internal/expiry` | 1 | `Lapse.lease` |

  `inspection`, `invariants`, `decay`, `config`, `time` and `failure` hold no id typed
  `String` and do not change: `inspection.views` and the checker pass ids between
  `model`'s lookups and compare them with `==`, and the types flow through.
- **The `String`s that stay hold no id.** After the change 32 lines under
  `gleam-double/src/` name `String`, and the pin lists them, each by its module and its
  trimmed text:

  | Module | Lines | What the `String` is |
  | --- | --- | --- |
  | `ids` | 5 | The five constructors' one field |
  | `internal/rules` | 7 | A rule's and a trigger's name, `Impossible`'s reason, the names of the rules that fired, `defect`'s reason |
  | `internal/core` | 5 | The names of the rules that fired, and the registry's names |
  | `failure` | 4 | A trigger's, a rule's and an invariant's names, and a defect's reason |
  | `refusals` | 4 | `ConfigRefused`'s invariants, and `action` in `LeaseFenced`, `RecordNotHeld` and `MarkConflict`, which is a trigger's name |
  | `invariants` | 2 | An invariant's name, and `check`'s result |
  | The root | 1 | `Outcome.fired` |
  | `answers` | 1 | `TickReport.fired` |
  | `config` | 1 | `broken`'s invariant names |
  | `internal/publish` | 1 | `revive`'s `name`, a rule's name |
  | `internal/settle` | 1 | `action`, a trigger's name |

  The four lines of `refusals` and `publish` keep their non-id `String` and gain id types
  in the same line; the pin's list gives them as they read after the change, for
  instance `LeaseFenced(action: String, consumer_id: ConsumerId, lease_id: LeaseId)`. At
  the base 128 lines name `String`. If `gleam format` splits one of the 32, the pin
  lists it as formatted, and the hand-back says so under Departures.
- **The engine writes the lease id's type qualified.** `rules` defines a `Plan`
  constructor named `LeaseId`, the request for a new id. So `rules.gleam` imports
  `pachinko_double/ids` without a name list and writes `ids.LeaseId`. Four modules
  import that request unqualified: `internal/pull.gleam`, `core_test.gleam`,
  `publish_test.gleam` and `rules_test.gleam`. Each writes the id's constructor
  `ids.LeaseId`, qualified, and takes the other four ids' constructors unqualified. The
  request keeps its name: no constructor is renamed.
- **An id is rendered by matching it.** At the base, 14 defect texts under
  `gleam-double/src/` concatenate an id into a `String`, over 28 lines with `<>`, in six
  modules: `publish` 3, `settle` 3, `lease_end` 3, `expiry` 2, `pull` 2 and `core` 1. Each
  keeps its text byte for byte. Where a text names an id, a plain `let` directly above
  it takes the `String` out by pattern, for instance `let PocketKey(key) = pocket_key`,
  and the text names `key`. A single-constructor pattern cannot fail, so this is not the
  `let assert` that shared rule 4 forbids. No text uses `string.inspect`, which would
  render `PocketKey("pk")`, and no module gains a function that renders an id. Ten test
  lines, in four modules, assert those texts (`lease_end_test` 4, `publish_test` 4,
  `settle_test` 1, `lease_test` 1). Nine give the text as one literal and do not
  change. The tenth, `lease_end_test.gleam:909`, builds it from a variable `key` with
  `<>`; it may match `key` out of a `RecordKey`, and the text it builds is the same.
- **Test support takes the typed ids too.** `test/support/harness.gleam` stands in for
  the root in almost every test, so its functions that pass an id to the root take the
  root's types, and what it reads back keeps them:

  | Harness item | After this ticket |
  | --- | --- |
  | `publisher_joins`, `consumer_joins`, `pull`, `batch`, `renew`, `ack`, `nack`, `release`, `lease_view` | Take the typed ids, in the root's order |
  | `publish`, `publish_as` | Take `PocketKey` and `RecordKey`, and `publish_as` a `PublisherId`; `publish`'s publisher is `PublisherId("p")`. The payload stays text, as the Rust fixture's is |
  | `Pulled` | `lease: LeaseId`, `pocket_key: PocketKey` |
  | `keys`, `marked_keys`, `order` | Return `List(RecordKey)`, `List(RecordKey)` and `List(PocketKey)` |
  | `pocket`, `record` | Take `PocketKey`, and `record` a `RecordKey` |
  | `lease_ids`, `drive` | `fn() -> LeaseId`, and `drive`'s `lease_id: LeaseId`; the fake hands out `LeaseId("lease-1")`, `LeaseId("lease-2")` and onward |

  The alternative, a harness that keeps `String`s and wraps them, is under Open points
  with its count.
- **The tests follow the compiler.** A test writes an id as its constructor around the
  literal it has today, `PocketKey("pk")`, and takes the constructors from
  `pachinko_double/ids` by name, as the tests take `model`'s. Two test modules have a
  local variable named `ids` (`owner_test.gleam:108` and `competing_test.gleam:87`),
  which would hide the module's qualifier in its scope; they import the constructors
  unqualified. A test that builds an id from a computed `String`, such as
  `"c" <> int.to_string(n)` in `competing_test.gleam`, wraps the result. At the base, at
  least 1,251 lines in 20 files under `gleam-double/test/` pass, build or read an id, and
  13 more call `rules.drive` with a lease-id function; with `interface_test.gleam` and
  `rules_test.gleam`, 22 of the 27 files under `gleam-double/test/` change. The five
  that do not are `coverage_test.gleam`, `pachinko_double_test.gleam`,
  `runner_test.gleam`, `support/runner.gleam` and `support/coverage.gleam`. The compiler
  is the check: a test that passes a `String` where an id is expected does not build.
- **Snapshot equality holds as it did.** Gleam's `==` compares a custom type's values
  field by field, so `PocketKey("pk") == PocketKey("pk")`, and two snapshots that held
  equal `String`s hold equal ids. A test that compares an id with a `String` does not
  compile, so no comparison can turn silently false. No code under `src/` orders by an
  id: every collection keeps creation order (G19), and nothing calls `string.compare` on
  a key. `inspection` and its views print nothing. The runner reports a crashed test's
  values with `string.inspect`, which will print `PocketKey("pk")` where it printed
  `"pk"`; no test asserts that text.
- **The guide follows the code.** In `gleam-double/README.md`:
  - Of the 23 fenced `gleam` blocks, 18 change. Two are excerpts of `src/`:
    `identity.gleam`'s `publisher_joins`, whose table becomes `Table(PublisherId)`, and
    `publish.gleam`'s `revive`, whose signature, guard annotation and `Impossible` text
    follow the decisions above. The other 16 are blocks of the walkthrough, which are
    `guide_test.gleam`'s text and change with it. `publish.gleam`'s `record` and
    `model.gleam`'s `RecordStatus` do not change, nor do the walkthrough's `Shown`,
    `fired` and closing check.
  - "Where to look" names `ids.gleam` first in its second row.
  - "Entities" gains, after its sentence on G09 and G10, one sentence on the ids: each id
    and key is a type of its own in `ids.gleam`, which wraps the specification's
    `String` and checks nothing.
  - The lease-id bullet of "Time, identity and inspection" gives
    `lease_id: fn() -> LeaseId`, and "A custom type" gives
    `Pocket(key: PocketKey("pk"), ...)` as its example.

  `just gleam-double` checks every block against its file; `guide_test.gleam`'s check is
  unchanged.
- **The root's header follows.** What a caller names when it sends a trigger or reads a
  reply is in `ids`, `config`, `time`, `model` and `refusals`, and the vocabulary is nine
  modules where it says eight.
- **No new spec note, and three sentences change.** The double stores the
  specification's `String` unchanged inside each type, and no rule reads an id
  differently, so the change interprets nothing (shared rule 5). Three sentences of
  `gleam-double/SPEC-NOTES.md` say an id is a `String`, and become:

  | Note | Today | After |
  | --- | --- | --- |
  | G05 | A lease id is a `String`, drawn from … | A lease id is a `LeaseId`, which wraps a `String`, drawn from … |
  | G09 | Each field holds the pocket's key or the consumer's id, a `String`. | Each field holds the pocket's key or the consumer's id, a `PocketKey` or a `ConsumerId`, each of which wraps a `String`. |
  | G10 | `Lease` has one more field, `id`, a `String`. | `Lease` has one more field, `id`, a `LeaseId`, which wraps a `String`. |

  The rest of each sentence stays.
- **What the pin gains**, in `gleam-double/test/interface_test.gleam`, with the unit
  GD08 fixed. Six new tests:
  - "ids holds its interface, and nothing marked": the five types, each with one
    constructor of its own name, none `@internal`.
  - "ids imports nothing": no line of `ids.gleam` starts with `import`.
  - A control, "a signature runs from its pub fn line to the line that ends with {", for
    a new reader `signatures`: a `pub fn` item's signature is its line and the lines
    after it, up to and including the first line of the code that ends with `{`; a line
    names `String` when the code holds `String` with no letter, digit or `_` directly
    before or after it. On hand-made text it names a one-line signature with
    `consumer: String`, a signature spread over lines whose inner line is
    `lease: gleam.String,`, and `pub fn c() -> fn() -> String {`. It does not name a
    `String` in a function's body, a private `fn`, a `StringTree` or a `String2`, or a
    `String` in a comment or a string.
  - "no pub fn of the root takes or returns a String": the root's twelve `pub fn`
    signatures name no `String`. At the base ten of them do, on 18 lines.
  - A control, "a line names String when its code holds the whole word", for a reader
    `string_lines`, which returns a module's trimmed code lines that name `String`. On
    hand-made text it takes a field of a private type and of an opaque one, a labelled
    argument `pocket pocket_key: String`, `lease_id: fn() -> String`, `Table(String)`
    and `key: gleam.String`. It leaves out `StringTree`, `MyString`, `String2`, a doc
    comment, a trailing comment and a string literal that holds `String`.
  - "the lines under src/ that name String are the 32 that hold no id": for every module
    under `gleam-double/src/`, `internal/` included, its `string_lines` are its rows of
    the pin's list, in the file's order, and a module the list does not name has none.

  Four tests change with the vocabulary:
  - `vocabulary_modules()` gains `ids`, so "src/pachinko_double/ holds the eight
    vocabulary modules and internal/" becomes "... the nine ...", and "no vocabulary
    module imports a module under internal/" reads `ids` too.
  - "answers imports only model and time" becomes "answers imports only ids, model and
    time".
  - The witness's allowed list gains `pachinko_double/ids`, and
    `gleam-double/test/guide_test.gleam` imports it.
  - The header's description of the assertions and of the parser's rules gains the two
    readers and the new assertions.

  The root's 17 items, GD10's marks and the six names `pub` only for the tests are
  unchanged.
- **One fact about Gleam is probed first**, at the step that first needs it, with
  `just gleam-double`, which builds with warnings as errors. It has not been run in
  this repository. The lease id's name clash needs no probe: the modules that meet it
  write `ids.LeaseId` qualified, as the decision above says.

  | Fact | Probe | If it does not hold |
  | --- | --- | --- |
  | A plain `let` destructures a single-constructor type, `let ConsumerId(text) = consumer`, with no warning | The first defect text to name a consumer's id, in step 4's `ConsumerId` stage | Write a `case` with one branch instead, and say so under Departures |

## Non-goals

- No behaviour change, and no change to what any trigger answers beyond the type of an
  id in it. Every defect text is byte for byte what it was.
- No test changes what it asserts. A test's lines change in four ways only: an id
  wrapped in its constructor, an import, a helper's id type, and a `let` that matches an
  id out or a constructor around a computed `String` where a test renders or computes
  one. Step 6 shows it.
- No validation, no function in `ids`, no opaque type and no alias. No other module
  gains a `pub fn` or a `pub type`.
- No field, label, argument, function or constructor is renamed, reordered, added or
  removed. `action`, the names of the rules and invariants, and the failure types' text
  stay `String`; payloads stay `BitArray` (G07).
- No change to `config`, `time`, `failure`, `inspection`, `invariants` or `decay`. If the
  compiler asks for one, the builder makes the smallest change and says so under
  Departures.
- No module moves, nothing under `internal/` becomes public, and no `@internal` mark is
  added or removed.
- No fenced block of `gleam-double/README.md` changes except the two excerpts of `src/`
  and the walkthrough blocks named above.
- No change to `gleam-double/gleam.toml`, `gleam-double/manifest.toml`, the `Justfile`
  or anything under `docs/specs/`, `docs/decisions/`, `crates/` or `double/`.
- No new spec note (shared rule 5); only the three sentences above change.

## Shared rules

Shared rules 1 to 6 bind this ticket as `.scratch/gleam-double/GD07-close-out.md` writes
them under "Shared rules (GD01 to GD07)", with rule 4's second sentence as
`.scratch/gleam-double/GD08-front-door.md` restates it under "Shared rules". They are not
copied here. Rule 3's coverage check stays strict at 93, 32 and 35. Under rule 1,
`double/pachinko_double/` and `double/tests/` stay unread.

Rule 4's "Failures are values" is untouched: a `let` that matches a single-constructor
type cannot fail, and no code under `src/` gains a `panic`, `todo`, `assert` or
`let assert`. Its "The clock and lease ids are injected" holds with the lease-id
function's new type: the owner is still started with it, and the fake under
`gleam-double/test/support/` still counts its draws.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/gleam-double/GD07-close-out.md` writes under "Adversarial review". Read them
with these substitutions:

- `<id>` is `GD12`, and `<n>` is 1: this ticket has one checkpoint, at step 8.
- The lenses are conformance and breakage. There is no `design` lens.
- The conformance brief names this ticket's path and its "Decisions fixed in this
  ticket", and `docs/specs/pachinko.allium` for the names and the declared types of the
  four ids it declares.
- `base` is the commit step 1 records. If the review runs before the work commit, the
  brief gives the uncommitted diff, `ids.gleam` read in full.
- The diff is large and mostly mechanical. The brief points each lens at step 6's
  residue file and at the files of `gleam-double/src/` first.
- A runner polls on `GD12-1/last-<lens>.md` or the process id, never the bare file name.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `gleam-double/src/pachinko_double/ids.gleam` | new | The five types, as "Decisions fixed" writes them |
| `gleam-double/src/pachinko_double.gleam` | shared, own lines | The ten functions' and `start`'s signatures, `Message`, `Owner`, the imports, the header |
| `gleam-double/src/pachinko_double/model.gleam`, `refusals.gleam`, `answers.gleam` | shared, own lines | The fields and lookups of the table above, and the import of `ids` |
| `gleam-double/src/pachinko_double/internal/rules.gleam`, `domain.gleam`, `core.gleam`, `identity.gleam`, `publish.gleam`, `pull.gleam`, `settle.gleam`, `lease_end.gleam`, `expiry.gleam`, `consuming.gleam` | shared, own lines | The table above; the defect texts' `let` lines; imports |
| `gleam-double/test/interface_test.gleam` | shared, own lines | What the pin gains |
| `gleam-double/test/support/harness.gleam` | shared, own lines | The harness table above |
| The other 20 changed modules under `gleam-double/test/` | shared, own lines | Ids wrapped, imports, helpers' id types |
| `gleam-double/README.md` | shared, own lines | The 18 blocks, "Where to look", "Entities", the lease-id bullet, the custom-type example |
| `gleam-double/SPEC-NOTES.md` | shared, own lines | The three sentences of G05, G09 and G10 |
| `.scratch/gleam-double/GD12-id-types.md` | ticket | `status:` and the hand-back notes |
| `.scratch/gleam-double/README.md` | shared, own lines | GD12's Status cell |

No other file under `gleam-double/src/` changes: `config`, `time`, `failure`,
`inspection`, `invariants` and `internal/decay` hold no id typed `String`.
`docs/project/repository-map.md` names no vocabulary module, so it does not change.

The counts, at `f56ece6caf0b7381e9b5671f9b4b38b378e1d75d`. The builder reruns each at
the base.

| What | Count | Command |
| --- | --- | --- |
| Lines under `src/` that type an id as `String` | 105, in 14 modules | 1 |
| Lines under `src/` that name `String` | 128 today; 32 after | 2 |
| Lines under `src/` with `<>` | 28, making 14 defect texts in 6 modules | 3 |
| Test lines that assert a defect text | 10, in 4 modules | 4 |
| The root's `pub fn`s, and those whose signature names `String` | 12; 10, on 18 lines | 5 |
| Test lines that pass, build or read an id | at least 1,251, in 20 files | 6 |
| Test lines that call `rules.drive` | 13, in 5 modules | 7 |
| Fenced `gleam` blocks of the guide, and lines in them that hold an id | 23; about 120 lines in 18 blocks | 8 |

The commands, run from the repository root. Command 1's pattern prints the 105 lines;
none is a comment.

```sh
# 1
p='\b(id|key|pocket|consumer|publisher|lease|nacked_by|publisher_id|consumer_id|pocket_key|record_key|lease_id): (fn\(\) -> )?String\b|\b(Table|Row)\(String\)|\bfn\(String\) -> (Plan|Step)\b'
rg -c "$p" gleam-double/src
# 2
rg -c '\bString\b' gleam-double/src
# 3
rg -n '<>' gleam-double/src
# 4
rg -n '"no (pocket|record|held|lease|active)|is not held|has no (deferred|nacked)|under lease |of consumer ' gleam-double/test
# 5
rg -c '^pub fn ' gleam-double/src/pachinko_double.gleam
awk '/^pub fn /{s=1} s{print} s && /\{$/{s=0}' gleam-double/src/pachinko_double.gleam | rg -c '\bString\b'
# 7
rg -c 'rules\.drive\(' gleam-double/test
```

Command 4 also prints one doc comment, `settle_test.gleam:1230`, which is not counted.
Command 6 is a proxy: it counts a line that calls a harness function, the root or a
`model` lookup that takes an id, a line whose labelled field is given a string literal,
and a line that builds a value whose constructor holds an id. It undercounts: a helper's
positional argument, such as `invariants_test.gleam`'s `with_pocket`, is not seen.

```sh
# 6
t='\b(harness|pachinko_double)\.(publisher_joins|consumer_joins|publish|publish_as|pull|batch|renew|ack|nack|release|lease_view|pocket|record|drive|keys|marked_keys|order)\(|\bmodel\.(pocket|record|lease|records_of|held_under|leases_of_pocket|leases_of_consumer)\(|\b(id|key|pocket|consumer|publisher|lease|nacked_by|publisher_id|consumer_id|pocket_key|record_key|lease_id): "|\b(Publisher|Consumer|Batch|BatchRecord|NoPocketAvailable|HeldView|PublisherAlreadyJoined|ConsumerAlreadyJoined|UnknownPublisher|EqualRankRefused|UnknownConsumer|PullRefused|LeaseFenced|RecordNotHeld|MarkConflict|ReleaseRefused|PublisherJoins|ConsumerJoins|Pull|Ack|Nack|Renew|Release|Lapse|Freeing|Handover|PocketTouched|Handed|Freed|LeaseLapsed)\('
rg -c "$t" gleam-double/test
```

Command 8 numbers the guide's `gleam` blocks and searches them with command 6's
pattern, widened by `(Table|Row)\(String\)` and `: String\b`. It prints 121 lines; the
one in block 4, `publish.gleam`'s `record`, does not change, which leaves about 120 in
18 blocks.

```sh
# 8
awk '/^```gleam/{inb=1;blk++;next} /^```$/{inb=0;next} inb{print blk": "$0}' gleam-double/README.md >| ai_tmp/gd/GD12/fences.txt
rg -c "$t"'|\b(Table|Row)\(String\)|: String\b' ai_tmp/gd/GD12/fences.txt
```

## Steps

Each red step quotes the failing test's name and its message verbatim, or the
compiler's error, headed by the full commit id and "with uncommitted changes". Evidence
goes under `ai_tmp/gd/GD12/`.

1. **Branch and baseline.** The series runs on `build-gleam-double` in this worktree
   (shared rule 6). Record the base's full commit id, which holds GD11. If `.pixi/`,
   `.tools/` or `gleam-double/build/packages/` is missing, run `just initialize` and
   `just gleam-double-deps` (network, authorised by this step). Run `just check` and
   `just gleam-double` on the untouched worktree. Quote each one's last line and the
   line `N tests run, N passed`, headed by the commit. The last recorded run at the base
   counts 353 (`ai_tmp/gd/series-review/fix2/ev-gleam-2.txt`). Rerun the eight commands
   of "Files touched" and save what they print. Then show the swap compiles today, on
   the untouched tree: a scratch `gleam-double/test/swap_test.gleam` whose one `pub fn`
   takes an `owner: Subject(Message)` and returns
   `pachinko_double.renew(owner, "lease-1", "c1")`, the consumer and the lease swapped.
   `just gleam-double` builds it, every test passes, and the run fails only with
   `swap_test.gleam is on disk and not listed in main`. Quote that and delete the file.
2. **Red: the pin.** Add to `interface_test.gleam` the six tests and the four changes of
   "What the pin gains". Run `just gleam-double-format` first, then `just gleam-double`.
   The two controls pass on hand-made text. These fail: the two `ids` tests, the layout
   test and "no vocabulary module imports a module under internal/", since `ids.gleam`
   cannot be read; "answers imports only ids, model and time"; "no pub fn of the root
   takes or returns a String", which names ten functions; and the `String` lines, which
   finds 128 lines against the list's 32. Quote each failure, and any other test that
   fails because `ids.gleam` is missing.
3. **Green: `ids.gleam`.** Write it as "Decisions fixed" gives it. `just gleam-double`
   passes except for three tests, which fail as in step 2: the root's signatures, the
   `String` lines, and "answers imports only ids, model and time", since `answers`
   names no id until step 4's `ConsumerId` stage and an unused import does not build.
4. **Red, then green, one id at a time**, in this order, so that each stage builds on
   its own: `PublisherId`, `ConsumerId`, `PocketKey`, `RecordKey`, `LeaseId`. In each
   stage:
   - **Red.** First change one call of the root in `partial_tick_keeps_what_ran`, in
     `gleam-double/test/owner_test.gleam`, to pass the new type. The lines are the
     base's; each stage's imports move them. `publisher_joins` at line 121,
     `consumer_joins` at line 123, the pocket key and then the record key of `publish`
     at line 128, and for the lease id the function at lines 110 to 115, made to return
     a `LeaseId`, which `start` takes at line 119. Run `just gleam-double` and quote the
     compiler's type mismatch.
   - **Green.** Change every field, parameter and call that holds that id, in
     `gleam-double/src/` and then in `gleam-double/test/`, with the defect texts' `let`
     lines where that id is rendered. Run `just gleam-double-format`, then
     `just gleam-double`. It passes except for the root's signatures and the `String`
     lines, whose failures name fewer functions and fewer lines at each stage, and,
     in the `PublisherId` stage, the `answers` imports test. Quote both counts.

   The `ConsumerId` stage probes the fact of "Decisions fixed", and turns the `answers`
   imports test green. After the `LeaseId` stage `just gleam-double` passes, with the
   baseline's count plus the pin's six new tests.
5. **The swap no longer compiles.** Write the scratch module of step 1 again with the
   typed swap, `pachinko_double.renew(owner, LeaseId("lease-1"), ConsumerId("c1"))`.
   `just gleam-double` stops on the compiler's type mismatch, before any test runs.
   Quote it and delete the file.
6. **Changed only by the ids.** Write `ai_tmp/gd/GD12/residue.py`, run with
   `python3 -I`. For each file under `gleam-double/src/` and `gleam-double/test/` that
   the diff from `base` changes, except `ids.gleam` and `interface_test.gleam`, it
   normalises the base's text and the tree's the same way, then compares them as token
   streams:
   - drop every import statement, from its `import` line through the line that closes
     its name list;
   - write `ids.X` as `X`, for each of the five names `X`;
   - write `X(a)` as `a`, where `a` is a string literal or a name with fields, such as
     `pocket_key` or `lease.id`;
   - write every other whole word `X` as `String`;
   - drop all whitespace, and a `,` directly before `)`, `]` or `}`.

   On the base's side the only such word is `rules`' request `LeaseId`, which both sides
   then read alike. Save each difference, with the lines of both texts it spans, as
   `ai_tmp/gd/GD12/residue.txt`. Expect two kinds: the defect texts' `let` lines under
   `src/`, and under `test/` a constructor around a computed `String` or a `let` that
   matches an id out. Classify every difference as one of the two; any other is a
   finding, to be fixed or reported under Departures. The script first runs three
   controls on hand-made pairs: a pair that differs only by wrapping shows no
   difference, a pair with two ids swapped shows one, and a pair with a literal changed
   shows one.
7. **Words.** Update the guide as "Decisions fixed" says, the root's header, and the
   three sentences of `gleam-double/SPEC-NOTES.md`. Run `just gleam-double`, which checks
   the guide's blocks against their files, and `just check-docs`.
8. **Adversarial review** (checkpoint 1: conformance and breakage, before the final
   commit). Questions for this checkpoint:
   - Does any field, parameter, constructor or `pub fn` under `gleam-double/src/` still
     hold an id as a `String`, or is any of the 32 lines not what its row says it is?
   - Is each of the five an open type with one constructor wrapping a `String`, in
     `ids.gleam`, defined nowhere else, and does `ids.gleam` import nothing?
   - Is any defect text different from what it was, by a byte?
   - Did any test change what it asserts? Is every entry of `residue.txt` one of the two
     kinds step 6 allows?
   - Did any field, label, argument, function or constructor change its name, its order
     or its count?
   - Is every fenced block of `gleam-double/README.md` that changed one of the 18, and
     does each still match its file?
   - Would the pin fail if one id went back to `String`, in the root, in `model`, in
     `refusals` or under `internal/`?
   - Does the coverage check still collect every test and stay strict at 93, 32 and 35?
   - Could a probe in Verification print nothing while its claim is false?
9. **Commit and hand back.** Commit the work on `branch:`. Rerun Verification at that
   commit and quote it, headed by the full commit id. Set `status: done` here and in
   `.scratch/gleam-double/README.md`, write the hand-back notes, and commit them as the
   record. **Stop** before pushing or opening a pull request: both need authorisation
   (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `ids.gleam` holds the five open types, each with one constructor wrapping a `String`,
  and imports nothing. No other module under `gleam-double/src/` defines a type of those
  names. The pin asserts both.
- No `pub fn` of the root names `String`, and the lines under `gleam-double/src/` that
  name `String` are exactly the 32 of the pin's list. The pin asserts both.
- A call of the root with two ids swapped does not compile (step 5), where the same
  call with `String`s did (step 1).
- Every defect text is byte for byte what it was. Of the ten test lines that assert
  them, the nine literal ones are unchanged, and the tenth builds the same text.
- Step 6's residue holds only the two kinds it allows.
- Under `gleam-double/src/`, the diff from the base touches exactly the 15 files of
  "Files touched".
- Steps 2 and 4 are each quoted red before they turn green, step 4 once for each of the
  five ids.
- `just gleam-double` exits 0 with the coverage check strict at 93, 32 and 35. Its test
  count is the baseline's plus the pin's six.
- The diff changes no fenced block of `gleam-double/README.md` beyond the 18, and every
  one of them matches its file.
- The checkpoint is recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script from the repository root; `set -eu` stops it at the first command
that fails. `base` is the commit step 1 records; set it before the script runs. The
script's first check stops it when `base` is unset or names no commit. Each probe that
passes by printing nothing follows its positive control on every spelling it must
catch. `sh` reads only the status of a pipeline's last command, so every command whose
failure matters runs alone in an assignment or last in its pipeline: `rg`, `git`,
`awk`, `sort` and `find` each fill a variable of their own, and only `printf` feeds a
pipe.

```sh
set -eu
git cat-file -e "$base^{commit}"   # base is set before the script runs
git rev-parse HEAD
just gleam-double
test -f gleam-double/build/packages/packages.toml   # the gate below runs with build/ populated

src=gleam-double/src/pachinko_double
root=gleam-double/src/pachinko_double.gleam
ids=$src/ids.gleam
test -f "$ids"

# ids imports nothing. The control counts an import line and not an indented one.
probe=$(printf 'import gleam/list\n  import gleam/option\n' | rg -c '^import ')
test "$probe" -eq 1
if rg -n '^import ' "$ids"; then exit 1; else test "$?" -eq 1; fi

# Each id type is defined once under src/, in ids.gleam. A definition is public,
# opaque, private, parameterised or an alias; the control counts all five and
# neither a longer name nor one with a digit.
pattern='^(pub )?(opaque )?type (PublisherId|ConsumerId|PocketKey|RecordKey|LeaseId)\b'
probe=$(printf 'pub type PocketKey {\npub opaque type LeaseId {\ntype RecordKey {\npub type ConsumerId(a1) {\npub type PublisherId =\npub type PocketKeys {\npub type LeaseId2 {\n' | rg -c "$pattern")
test "$probe" -eq 5
homes=$(rg -c "$pattern" gleam-double/src)
test "$homes" = "$ids:5"
# ... and each is open, with one constructor that wraps a String.
for name in PublisherId ConsumerId PocketKey RecordKey LeaseId; do
  rg -q "^pub type $name \\{\$" "$ids"
  one=$(rg -c "^  $name\\(String\\)\$" "$ids")
  test "$one" -eq 1
done
constructors=$(rg -c '^  [A-Z]' "$ids")
test "$constructors" -eq 5

# No pub fn signature of the root names String. A signature runs from its
# `pub fn` line to the first line that ends with `{`. The control's three
# signatures each name String once, in one line, in an inner line and as a
# function's result; a body, a private fn and StringTree name none.
signatures='/^pub fn /{s=1} s{print} s && /\{$/{s=0}'
control=$(printf 'pub fn a(owner: Subject(Message), consumer: String) -> Reply {\n  body(String)\n}\npub fn b(\n  owner: Subject(Message),\n  lease: gleam.String,\n) -> Reply {\n  body(String)\n}\npub fn c() -> fn() -> String {\nfn d(x: String) -> Nil {\npub fn e(t: StringTree) -> Nil {\n' | awk "$signatures")
probe=$(printf '%s\n' "$control" | rg -c '\bString\b')
test "$probe" -eq 3
sigs=$(awk "$signatures" "$root")
fns=$(printf '%s\n' "$sigs" | rg -c '^pub fn ')
test "$fns" -eq 12
if printf '%s\n' "$sigs" | rg -n '\bString\b'; then exit 1; else test "$?" -eq 1; fi

# The lines under src/ that name String, outside a whole-line comment, are 32,
# by module as "Decisions fixed" counts them. The word control takes a bare, a
# qualified and a nested String, and neither StringTree, MyString nor String2;
# the comment control drops a doc comment and an indented comment.
probe=$(printf 'consumer: String,\nlease_id: fn() -> String,\nkey: gleam.String,\nTable(String)\nt: StringTree,\nx: MyString,\ny: String2,\n' | rg -c '\bString\b')
test "$probe" -eq 4
comment='^[^:]+:[0-9]+:[[:space:]]*//'
probe=$(printf 'a.gleam:3:/// a String\na.gleam:4:  // String\na.gleam:5:  x: String,\n' | rg -vc "$comment")
test "$probe" -eq 1
found=$(rg -n --with-filename '\bString\b' gleam-double/src)
code=$(printf '%s\n' "$found" | rg -v "$comment")
counts=$(printf '%s\n' "$code" | awk -F: '{n[$1]++} END {for (f in n) print f ":" n[f]}')
sorted=$(printf '%s\n' "$counts" | LC_ALL=C sort)
joined=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$joined" = "$root:1 $src/answers.gleam:1 $src/config.gleam:1 $src/failure.gleam:4 $src/ids.gleam:5 $src/internal/core.gleam:5 $src/internal/publish.gleam:1 $src/internal/rules.gleam:7 $src/internal/settle.gleam:1 $src/invariants.gleam:2 $src/refusals.gleam:4 "

# No defect text renders an id with string.inspect. The control takes it
# qualified and in a pipe.
pattern='\bstring\.inspect\b'
probe=$(printf 'Impossible("no pocket " <> string.inspect(key))\nkey |> string.inspect\n' | rg -c "$pattern")
test "$probe" -eq 2
if rg -n "$pattern" gleam-double/src; then exit 1; else test "$?" -eq 1; fi
# ... and the tests still assert the texts as they were.
rg -q -F '"no held record q under lease lease-1"' gleam-double/test/lease_end_test.gleam
rg -q -F '"nacked record m has no nacked_at"' gleam-double/test/lease_end_test.gleam
rg -q -F '" has no deferred_position"' gleam-double/test/lease_end_test.gleam
rg -q -F '"no held record k3 under lease lease-1"' gleam-double/test/settle_test.gleam
rg -q -F '"no record a in pocket pk"' gleam-double/test/publish_test.gleam
rg -q -F '"record a in pocket pk is not held"' gleam-double/test/publish_test.gleam
rg -q -F '"no pocket nowhere"' gleam-double/test/publish_test.gleam
rg -q -F '"no active lease lease-0 of consumer c1 on a leased pocket"' gleam-double/test/lease_test.gleam

# The walkthrough imports only what the witness allows, ids among them. The
# control holds an engine import plain, with names and under an alias, the
# support modules by their old and new names, and ids with names and with a
# digit.
allowed='^import (gleam/[a-z_/]+|simplifile|support/(harness|runner)|pachinko_double|pachinko_double/(ids|config|time|model|refusals|failure|answers|inspection|invariants))(\.\{| as |$)'
probe=$(printf 'import pachinko_double/internal/core.{TickReport}\nimport support/harness.{type Double}\nimport pachinko_double/internal/domain\nimport pachinko_double.{Outcome}\nimport pachinko_double/internal/core as engine\nimport pachinko_double/answers as answer\nimport pachinko_double/ids.{ConsumerId, PocketKey}\nimport pachinko_double/ids2\nimport harness\n' | rg -c "$allowed")
test "$probe" -eq 4
all=$(rg -c '^import ' gleam-double/test/guide_test.gleam)
ok=$(rg -c "$allowed" gleam-double/test/guide_test.gleam)
test "$all" -eq "$ok"
rg -q '^import pachinko_double/ids\b' gleam-double/test/guide_test.gleam

# The guide and the notes no longer give an id as a String. Each pattern's
# control is the base's text, which holds it; the guide's control also holds
# each alternative once, and an id already wrapped, which it must not take.
pattern='fn\(\) -> String|\b(Table|Row)\(String\)|\b(id|key|pocket|consumer|publisher|lease|nacked_by): "|\bharness\.[a-z_]+\(double, "'
probe=$(printf 'lease_id: fn() -> String\nTable(String)\nRow(String)\nid: "p"\nkey: "pk"\npocket: "pk"\nconsumer: "c1"\npublisher: "p"\nlease: "lease-1"\nnacked_by: "c1"\nharness.batch(double, "c2", 10)\nkey: PocketKey("pk")\nmonkey: "x"\nharness.publish(double, PocketKey("pk"), RecordKey("a"), 1, "v1")\n' | rg -c "$pattern")
test "$probe" -eq 11
old=$(git show "$base:gleam-double/README.md")
probe=$(printf '%s\n' "$old" | rg -c "$pattern")
test "$probe" -gt 0
if rg -n "$pattern" gleam-double/README.md; then exit 1; else test "$?" -eq 1; fi
old=$(git show "$base:gleam-double/SPEC-NOTES.md")
for text in 'A lease id is a `String`' 'id, a `String`.' '`id`, a `String`'; do
  probe=$(printf '%s\n' "$old" | rg -c -F "$text")
  test "$probe" -eq 1
  if rg -n -F "$text" gleam-double/SPEC-NOTES.md; then exit 1; else test "$?" -eq 1; fi
done

# The files changed under src/ are exactly the fifteen. The control: the same
# command names a file that did change.
seen=$(git diff --name-only "$base" -- "$src/model.gleam")
test "$seen" = "$src/model.gleam"
names=$(git diff --name-only "$base" -- gleam-double/src)
sorted=$(printf '%s\n' "$names" | LC_ALL=C sort)
changed=$(printf '%s\n' "$sorted" | tr '\n' ' ')
test "$changed" = "$root $src/answers.gleam $src/ids.gleam $src/internal/consuming.gleam $src/internal/core.gleam $src/internal/domain.gleam $src/internal/expiry.gleam $src/internal/identity.gleam $src/internal/lease_end.gleam $src/internal/publish.gleam $src/internal/pull.gleam $src/internal/rules.gleam $src/internal/settle.gleam $src/model.gleam $src/refusals.gleam "
pins=$(git diff --name-only "$base" -- gleam-double/gleam.toml gleam-double/manifest.toml)
test -z "$pins"

dumps=$(find gleam-double -name erl_crash.dump -print)
test -z "$dumps"
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

The `String` count reads each line `rg` prints as `path:line:text` and leaves out a line
whose text starts with `//` after its indentation, so a doc comment that names `String`,
such as `ids.gleam`'s header, does not count; a trailing comment would, and fails the
probe closed. The pin reads the same lines from each module's code, without comments or
strings, and compares their text, not their count. The guide's pattern takes a harness
call whose first id is a string literal, so it holds only while the harness takes typed
ids. The changed-files probe runs after the work commit, when `ids.gleam` is tracked;
run before it, `git diff` cannot name an untracked file, and the run must add
`git ls-files --others --exclude-standard -- gleam-double/src` and say so. The builder
runs each control first, and corrects a pattern, saying so under Departures, if one does
not behave as written. No probe reads a diff's body: the changed-files probe reads names
only, and step 6 compares texts, not patches. A probe added later that reads a patch
takes `git diff --text` and counts a file header only before its file's first `@@`.

## Hand-back notes

Written at the end, in this order: base, branch and commits; the baseline; what landed;
the red steps, one per stage of step 4; the swap; the residue; the checkpoint's record;
Verification at the work commit; departures; and what was not run.

## Open points

- **The harness could keep `String`s.** This ticket gives
  `test/support/harness.gleam` the typed ids, so that a test reads in the caller's
  vocabulary and a swapped argument in a test does not compile either. The alternative
  keeps the harness's `String` parameters, which mirror the Rust fixture's, and wraps
  them at the call of the root. It would leave about 626 harness call lines in 16 test
  modules and 53 lines that compare `harness.keys`, `marked_keys` or `order` with a list
  of literals as they are, roughly half the test diff. The tests would then mix the two
  vocabularies: a call with `"c1"`, and an expected refusal or snapshot with
  `ConsumerId("c1")`. The maintainer can choose it before the ticket starts; the guide's
  probe in Verification then drops its harness alternative.
- **A spec note, or none.** G07 recorded a type that departs from the specification's
  `String`: the payload, which the double stores as a `BitArray`. This ticket writes no
  note for the ids, because each type holds the specification's `String` unchanged,
  where G07's holds other values. If the maintainer reads the two alike, a short note
  says that each id is a type of its own that wraps the `String`, with no question.
- **No id is checked.** An empty `PocketKey("")` is as valid as any other, as the
  specification states no format. A format would be a product decision, and its
  validation a behaviour change.
- **The test diff is large.** Most of it is mechanical, and step 6's residue is what
  the review reads to see that nothing else moved. Splitting the ticket by id would give
  five smaller diffs, each of which builds; the maintainer chose one ticket, and step 4
  stages it by id within it.
