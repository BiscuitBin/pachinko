---
id: GD12
title: "Gleam double: a type of its own for each id"
status: done
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
- **Test support takes the typed ids too.** The maintainer ruled so on 2026-11-23, as
  this ticket drafted it. `test/support/harness.gleam` stands in for the root in almost
  every test, so its functions that pass an id to the root take the root's types, and
  what it reads back keeps them:

  | Harness item | After this ticket |
  | --- | --- |
  | `publisher_joins`, `consumer_joins`, `pull`, `batch`, `renew`, `ack`, `nack`, `release`, `lease_view` | Take the typed ids, in the root's order |
  | `publish`, `publish_as` | Take `PocketKey` and `RecordKey`, and `publish_as` a `PublisherId`; `publish`'s publisher is `PublisherId("p")`. The payload stays text, as the Rust fixture's is |
  | `Pulled` | `lease: LeaseId`, `pocket_key: PocketKey` |
  | `keys`, `marked_keys`, `order` | Return `List(RecordKey)`, `List(RecordKey)` and `List(PocketKey)` |
  | `pocket`, `record` | Take `PocketKey`, and `record` a `RecordKey` |
  | `lease_ids`, `drive` | `fn() -> LeaseId`, and `drive`'s `lease_id: LeaseId`; the fake hands out `LeaseId("lease-1")`, `LeaseId("lease-2")` and onward |

- **One short spec note, G68, with no question.** The maintainer ruled on 2026-11-23
  that the ids get a note for parity with G07, which records the payload's `BitArray`:
  each id is a type of its own that wraps the specification's `String`. Its text and
  its place are fixed in the second group.
- **No validation in this ticket, and the ids' format is a finding.** The maintainer
  ruled on 2026-11-23 that no id is checked here. Whether an id has a format is raised
  as a "The maintainer decides" finding in `gleam-double/SPEC-NOTES.md`, whose text the
  second group fixes. The specification is untouched.

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
    `guide_test.gleam`'s text and change with it. Each block changes in the stage of
    step 4 that changes the lines it copies; the prose below changes at step 7.
    `publish.gleam`'s `record` and
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
- **One new spec note, G68, and three sentences change.** The double stores the
  specification's `String` unchanged inside each type, and no rule reads an id
  differently, so the change interprets nothing (shared rule 5). The note is written for
  parity with G07, by the maintainer's ruling, not because the change interprets the
  specification. G68 is filed last under "Where it raises what no `N`-note holds" in
  `gleam-double/SPEC-NOTES.md`, after G66 and before "The `impl-choice` rows". Its text
  is fixed here, so that the builder writes no prose of their own:

  ```markdown
  ### G68: each id and key is a type of its own, which wraps the specification's `String`

  - **Ticket:** GD12.
  - **Concerns:** `Publisher.id`, `Consumer.id`, `Pocket.key`, `Record.key`, the `key` of
    `SupersededRecord` and `DeadLetter`, the references of G09, the lease's id of G10, and
    every trigger that carries one of them.
  - **The specification says:** the four ids it declares are each typed `String`, with no
    format: no length, no character set, nothing about an empty one. It declares the lease
    no identity (G10).
  - **The double does:**
    - `ids.gleam` defines five open types, `PublisherId`, `ConsumerId`, `PocketKey`,
      `RecordKey` and `LeaseId`, each with one constructor that wraps a `String` and checks
      nothing. Every field, parameter and reply that holds an id takes its type, so passing
      one id where another is expected does not compile.
    - The `String` inside is stored unchanged: the caller's, or for a lease id the one in
      the `LeaseId` that the owner's lease-id function returns (G10). No rule reads it: a
      guard or a lookup compares two ids with `==`, and a defect text takes the `String`
      out by matching it. Like G07's payload, this is a type other than the one the
      specification declares, recorded so that a reader of the specification knows why.
    - The pin in `interface_test.gleam` holds that no `pub fn` of the root names `String`,
      and that the lines under `src/` that name it are the 32 that hold no id.
  - **`N`-note:** no `N`-note. N06 holds a reference as the key, a string; this note
    changes the key's type, not what a reference holds.
  - **Question:** none. Whether an id has a format is item 9 of "What this double raises
    that no `N`-note holds".
  ```

  Three sentences of `gleam-double/SPEC-NOTES.md` say an id is a `String`, and become:

  | Note | Today | After |
  | --- | --- | --- |
  | G05 | A lease id is a `String`, drawn from … | A lease id is a `LeaseId`, which wraps a `String`, drawn from … |
  | G09 | Each field holds the pocket's key or the consumer's id, a `String`. | Each field holds the pocket's key or the consumer's id, a `PocketKey` or a `ConsumerId`, each of which wraps a `String`. |
  | G10 | `Lease` has one more field, `id`, a `String`. | `Lease` has one more field, `id`, a `LeaseId`, which wraps a `String`. |

  The rest of each sentence stays. The note count follows: "The 67 notes follow" becomes
  "The 68 notes follow", and the heading table's row "Raises what no `N`-note holds"
  counts 22 where it counts 21.
- **The ids' format is a finding, and nothing checks it.** By the maintainer's ruling, it
  is item 9 of the numbered list under "What this double raises that no `N`-note holds",
  in the findings of `gleam-double/SPEC-NOTES.md`, after item 8. It cites
  `docs/specs/pachinko.allium`, which declares no format, and G68. It is not listed under
  "Open questions left untouched", which lists only the specification's own open
  questions. The sentence after the list, "G44, G51, G53, G55 and G66 record a reading
  and leave no question.", does not change: the finding cites G68. The specification is
  untouched. Its text is fixed here:

  ```markdown
  9. **The ids' format.** The specification declares a publisher's id, a consumer's id, a
     pocket key and a record key each as a `String` and states no format for any of them
     (`docs/specs/pachinko.allium`); the lease's id is the double's own (G10). The double
     wraps each in a type of its own and checks nothing, so an empty key is as valid as any
     other (G68). **The maintainer decides:** do publisher ids, consumer ids, pocket keys,
     record keys and lease ids have a format, such as non-empty?
  ```

- **What the pin gains**, in `gleam-double/test/interface_test.gleam`, with the unit
  GD08 fixed. Eight new tests:
  - "ids holds its interface, and nothing marked": the five types, each with one
    constructor of its own name, none `@internal`.
  - "ids imports nothing": no line of `ids.gleam` starts with `import`.
  - A control, "a type is defined by a type line at column 0, public, opaque or
    private", for a reader `defined_types`, which returns the names a module's code
    defines as types: from each line that starts at column 0 with `type`, `pub type`
    or `pub opaque type` and a space, the word that follows. On hand-made text it names
    a public type with a body, an alias, an opaque type, a private type and
    `pub type ConsumerId(a) {`, whose name ends at the `(`. It does not name a
    constructor indented by two spaces, such as `rules`'
    `LeaseId(next: fn(ids.LeaseId) -> Plan(state, event, answer))`, a `pub fn`, or a
    `type` line in a comment or a string.
  - "only ids defines a type named like an id": `ids`' `defined_types` are the five,
    and no other module under `gleam-double/src/`, `internal/` included, defines one of
    their names, public or private, as a type or an alias. `items` reads only `pub`
    lines, so it cannot hold this.
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
    `gleam-double/test/guide_test.gleam` imports it from step 4's `PublisherId` stage.
  - The header's description of the assertions and of the parser's rules gains the three
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
- No spec note beyond G68, and no finding beyond item 9. No other sentence of
  `gleam-double/SPEC-NOTES.md` changes beyond the three above, the note count and the
  heading table's count.

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
| `gleam-double/SPEC-NOTES.md` | shared, own lines | The three sentences of G05, G09 and G10; the new note G68; item 9 of the findings; the note count and the heading table |
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
2. **Red: the pin.** Add to `interface_test.gleam` the eight tests and the four changes
   of "What the pin gains". Run `just gleam-double-format` first, then
   `just gleam-double`. The three controls pass on hand-made text. These fail: the three
   tests that read `ids` by name, the layout test and "no vocabulary module imports a
   module under internal/", since `ids.gleam` cannot be read; "answers imports only
   ids, model and time"; "no pub fn of the root takes or returns a String", which names
   ten functions; and the `String` lines, which finds 128 lines against the list's 32.
   Quote each failure, and any other test that fails because `ids.gleam` is missing.
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
     lines where that id is rendered. Run `just gleam-double-format`. Then copy into
     `gleam-double/README.md`, verbatim as formatted, each fenced block that copies a
     line this stage changed, in `guide_test.gleam` or in an excerpted source, so that
     the guide's drift test, "every gleam block of the guide is verbatim in its source,
     and the setting's, the common start's and each scenario's section shows its whole
     function", stays green. The `PublisherId` stage changes two blocks:
     `identity.gleam`'s excerpt and the setting's block, which holds
     `harness.publisher_joins(double, "p")`. It is also the stage at which
     `guide_test.gleam` gains its import of `pachinko_double/ids`: before step 3 the
     module cannot be found, and before the walkthrough names an id the import is
     unused, which does not build. Then run `just gleam-double`. It passes except for
     the root's signatures and the `String` lines, and, in the `PublisherId` stage, the
     `answers` imports test. The `String` lines fall at each stage. The root's
     signatures name, stage by stage, 9, 6, 6, 5 and then no functions, on 16, 9, 8, 5
     and then no lines, against 10 on 18 at the base: the `PocketKey` stage leaves six
     functions, because `publish` still takes its record key as a `String`. Quote the
     three counts: the root's functions and lines, and the `String` lines.

   The `ConsumerId` stage probes the fact of "Decisions fixed", and turns the `answers`
   imports test green. After the `LeaseId` stage `just gleam-double` passes, with the
   baseline's count plus the pin's eight new tests.
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
7. **Words.** Update the guide's prose as "Decisions fixed" says: "Where to look",
   "Entities", the lease-id bullet and "A custom type"; step 4 has changed its blocks.
   Update the root's header. In
   `gleam-double/SPEC-NOTES.md`, change the three sentences of G05, G09 and G10, add G68
   last under "Where it raises what no `N`-note holds" and item 9 after item 8 of "What
   this double raises that no `N`-note holds", each as "Decisions fixed" gives it, and
   make "The 67 notes follow" read 68 and the heading table's 21 read 22. Run
   `just gleam-double`, which checks the guide's blocks against their files, and
   `just check-docs`.
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
   - Do G68 and item 9 read as "Decisions fixed" gives them, and does no other sentence
     of `gleam-double/SPEC-NOTES.md` change beyond the three of G05, G09 and G10, the
     note count and the heading table's count?
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
  count is the baseline's plus the pin's eight.
- The diff changes no fenced block of `gleam-double/README.md` beyond the 18, and every
  one of them matches its file.
- `gleam-double/SPEC-NOTES.md` holds G68, last under "Where it raises what no `N`-note
  holds", and item 9 with its "The maintainer decides" question, each as "Decisions
  fixed" gives it. It counts "The 68 notes", and its heading table gives 22 for "Raises
  what no `N`-note holds". Nothing under `docs/specs/` changes.
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
`awk`, `sort`, `find` and `cat` each fill a variable of their own, and only `printf`
feeds a pipe. No probe passes on an empty pipe: one that reads a variable through
`printf` counts a line it knows is there, or compares what it read with a fixed text.

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
# The signatures are read from a variable, so the probe appends one line that
# names String and counts exactly that one: a printf that writes nothing gives
# rg no line, and rg's status 1 stops the assignment, where `if ... | rg` would
# read the empty pipe as absence.
named=$(printf '%s\nString\n' "$sigs" | rg -c '\bString\b')
test "$named" -eq 1

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
# ... and the tests still assert the texts as they were: each of the nine
# literal lines by counting its literal in its file, as often as at the base,
# so that a changed copy of a repeated text is seen, and the tenth by its whole
# line, the prefix, one name and the suffix. The literal's control counts it
# twice and refuses another record, a longer pocket and the text unquoted; the
# line's control takes `key` and another name, and refuses a changed prefix, a
# changed suffix and a literal in place of the name.
probe=$(printf '  EffectFailed(r, "no record a in pocket pk"),\n  EffectFailed(s, "no record a in pocket pk"),\n  EffectFailed(r, "no record b in pocket pk"),\n  EffectFailed(r, "no record a in pocket pk2"),\n  // no record a in pocket pk\n' | rg -c -F '"no record a in pocket pk"')
test "$probe" -eq 2
asserted() {
  n=$(rg -c -F "$3" "gleam-double/test/$1.gleam")
  test "$n" -eq "$2"
}
asserted lease_end_test 1 '"no held record q under lease lease-1"'
asserted lease_end_test 2 '"nacked record m has no nacked_at"'
asserted settle_test 1 '"no held record k3 under lease lease-1"'
asserted publish_test 2 '"no record a in pocket pk"'
asserted publish_test 1 '"record a in pocket pk is not held"'
asserted publish_test 1 '"no pocket nowhere"'
asserted lease_test 1 '"no active lease lease-0 of consumer c1 on a leased pocket"'
pattern='^\s*"held record " <> [a-z_][a-z0-9_]* <> " has no deferred_position",$'
probe=$(printf '            "held record " <> key <> " has no deferred_position",\n  "held record " <> text2 <> " has no deferred_position",\n  "held  record " <> key <> " has no deferred_position",\n  "held record " <> key <> " has no deferred_positions",\n  "held record " <> "k" <> " has no deferred_position",\n' | rg -c "$pattern")
test "$probe" -eq 2
built=$(rg -c "$pattern" gleam-double/test/lease_end_test.gleam)
test "$built" -eq 1

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

# The notes hold G68 once, as a heading of its own. The control counts it and
# neither a longer id, a deeper heading nor another note.
pattern='^### G68: '
probe=$(printf '### G68: x\n### G680: x\n#### G68: x\n### G67: x\n' | rg -c "$pattern")
test "$probe" -eq 1
notes=$(rg -c "$pattern" gleam-double/SPEC-NOTES.md)
test "$notes" -eq 1
# ... and item 9 asks its question once, on one line or wrapped. rg -c exits 1
# when nothing matches, which stops set -eu in the assignment; that is a failure
# here too, since each count must be at least 1. The control holds the question
# on one line, wrapped as item 9 wraps it, and without its last words.
pattern='\*\*The\s+maintainer\s+decides:\*\*\s+do\s+publisher\s+ids,\s+consumer\s+ids,\s+pocket\s+keys,\s+record\s+keys\s+and\s+lease\s+ids\s+have\s+a\s+format,\s+such\s+as\s+non-empty\?'
probe=$(printf '**The maintainer decides:** do publisher ids, consumer ids, pocket keys, record keys and lease ids have a format, such as non-empty?\n   other (G68). **The maintainer decides:** do publisher ids, consumer ids, pocket keys,\n   record keys and lease ids have a format, such as non-empty?\n**The maintainer decides:** do publisher ids, consumer ids, pocket keys, record keys and lease ids have a format?\n' | rg -U -c "$pattern")
test "$probe" -eq 2
asked=$(rg -U -c "$pattern" gleam-double/SPEC-NOTES.md)
test "$asked" -eq 1
# ... and the counts follow: 68 notes, 22 of them under no N-note. The control
# holds each count and the base's, and each pattern takes only its own.
counts='The 68 notes follow\nThe 67 notes follow\n| Raises what no `N`-note holds | 22 |\n| Raises what no `N`-note holds | 21 |\n'
for text in 'The 68 notes follow' 'The 67 notes follow' '| Raises what no `N`-note holds | 22 |'; do
  probe=$(printf "$counts" | rg -c -F "$text")
  test "$probe" -eq 1
done
rg -q -F 'The 68 notes follow' gleam-double/SPEC-NOTES.md
if rg -n -F 'The 67 notes follow' gleam-double/SPEC-NOTES.md; then exit 1; else test "$?" -eq 1; fi
rg -q -F '| Raises what no `N`-note holds | 22 |' gleam-double/SPEC-NOTES.md
# ... and the notes hold what the counts say, where they belong. The file holds
# 68 notes; its control counts a note of any id and neither a deeper heading
# nor a heading with no number. `placed` slices "Where it raises what no
# `N`-note holds" up to the next `## ` heading, and prints how many notes it
# holds and the last one's id. `listed` slices "What this double raises that no
# `N`-note holds" up to the next heading, and prints how many items it numbers,
# how many of them are out of order, and the last one's title. Their controls
# hold a note in another section, a deeper heading, an item numbered 10 and an
# item after item 9's, and then each as it should be.
pattern='^### G[0-9]+: '
probe=$(printf '### G68: x\n### G7: y\n#### G1: z\n### G: w\n' | rg -c "$pattern")
test "$probe" -eq 2
total=$(rg -c "$pattern" gleam-double/SPEC-NOTES.md)
test "$total" -eq 68
placed='/^## /{s=($0 == "## Where it raises what no `N`-note holds")} s && /^### G[0-9]+: /{n++; last=$2} END {print n+0, last}'
probe=$(printf '%s\n' '## Where the second reading agrees with an `N`-note' '### G68: x' '## Where it raises what no `N`-note holds' '### G62: y' '#### G69: z' '### G66: w' '## The `impl-choice` rows' '### G70: v' | awk "$placed")
test "$probe" = "2 G66:"
probe=$(printf '%s\n' '## Where it raises what no `N`-note holds' '### G66: w' '### G68: x' '## The `impl-choice` rows' | awk "$placed")
test "$probe" = "2 G68:"
probe=$(awk "$placed" gleam-double/SPEC-NOTES.md)
test "$probe" = "22 G68:"
listed='/^##+ /{s=($0 == "### What this double raises that no `N`-note holds")} s && /^[0-9]+\. /{n++; if ($1 != n ".") bad++; last=substr($0, length($1) + 2, 20)} END {print n+0, bad+0, last}'
probe=$(printf '%s\n' '### What this double raises that no `N`-note holds' '1. **a.** x' "10. **The ids' format.** y" '3. **b.** z' '### What the actor showed about the serialized stream' '4. **c.** w' | awk "$listed")
test "$probe" = "3 1 **b.** z"
probe=$(printf '%s\n' '### What this double raises that no `N`-note holds' '1. **a.** x' '   wrapped' "2. **The ids' format.** y" '#### Deeper' '3. **c.** w' | awk "$listed")
test "$probe" = "2 0 **The ids' format.**"
probe=$(awk "$listed" gleam-double/SPEC-NOTES.md)
test "$probe" = "9 0 **The ids' format.**"
# ... and G68 and item 9 read word for word as "Decisions fixed" gives them.
# `fixed` reads this ticket's two `markdown` blocks, without their two spaces
# of indentation, then, after a marker line, the notes, and prints how many
# blocks it read and how many of them the notes hold as a run of whole lines.
# The control's notes hold its first block and then its second, with one word
# changed and as given.
fixed='
/^=== the notes$/ { notes = 1; next }
!notes && /^  ```markdown$/ { b++; inb = 1; next }
!notes && inb && /^  ```$/ { inb = 0; next }
!notes && inb { blk[b] = blk[b] substr($0, 3) "\n"; next }
notes { text = text $0 "\n" }
END { for (i = 1; i <= b; i++) if (index("\n" text, "\n" blk[i])) f++; print b + 0, f + 0 }'
for last in 't 2 1' 'u 2 2'; do
  probe=$(printf '%s\n' '  ```markdown' '  ### G68: x' '' '  - y' '  ```' '  ```gleam' '  z' '  ```' '  ```markdown' '  9. **w.** v' '     u' '  ```' '=== the notes' '### G67: a' '### G68: x' '' '- y' '' '9. **w.** v' "   ${last%% *}" | awk "$fixed")
  test "$probe" = "${last#* }"
done
ticket=$(cat .scratch/gleam-double/GD12-id-types.md)
text=$(cat gleam-double/SPEC-NOTES.md)
probe=$(printf '%s\n=== the notes\n%s\n' "$ticket" "$text" | awk "$fixed")
test "$probe" = "2 2"

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
call whose first id is a string literal, which the typed harness no longer accepts. The
changed-files probe runs after the work commit, when `ids.gleam` is tracked;
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

Evidence files are under `ai_tmp/gd/GD12/`, the checkpoint's record under
`ai_tmp/review/GD12-1/` and the plan review's under `ai_tmp/review/GD12-plan/`. None is
committed. Most of the implementer's records of steps 2 to 7 open with the line
`547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes`, most of them then a
note on what the run is. The baseline's two records, `ev-counts-base.txt`, `fences.txt`,
`ev-swap-strings.txt`, `ev-defect-literals.txt`, `format-diff.txt` and the 47
`just gleam-double-format` records name no commit. `ev-verification-at-work.txt` opens
`$ sh ai_tmp/gd/GD12/verification.sh` and names its commit through `git rev-parse HEAD`.
In every quote below the recipe's echo of its own command lines is left out, an absolute
path is cut to its part under the repository with `...`, the runner's coloured
`runtime error` lines are left out, and `...` marks other lines left out or a line cut
short: a `FAIL` line is cut after the test's name. Times are UTC on 2026-11-24, read from
the files' modification times, which are each file's last write. The base commit's date
is 2026-11-23T23:11:02-08:00 (07:11:02Z), the work commit's 2026-11-23T23:59:31-08:00
(07:59:31Z). Where a fact the record should hold was never saved, the text says "not
recorded"; where it rests only on the lead's message, it says "by the lead's account".

### 1. Base, branch and commits

- Branch: `build-gleam-double`, in this worktree (shared rule 6).
- After `f56ece6`, the commit Context names and PR #53's head, three commits touch this
  ticket alone, the first with the series index: `02236a1` adds it; `d604e23` records
  the maintainer's three rulings of 2026-11-23; `547c34b` applies the plan review's
  fixes (section 7).
- Base commit: `547c34b4dbad59558b88b7fb46ed6e373f01bb35` (`base.txt`, last written at
  07:11:04Z). It holds GD11, the series review's fixes and this ticket as the plan review
  left it.
- Work commit: `fc5fbac1f1527dfe9c0d5b7895120c8fe712826f` (`work-commit.txt`, 07:59:33Z),
  tree `509fbb895a0407498ebb55b256a6785289491a84`, titled "Give each of the Gleam
  double's ids a type of its own (GD12)". It touches 39 files (`git show --stat`, 4154
  insertions and 2208 deletions): 15 under `gleam-double/src/`, the new `ids.gleam` and
  the 14 modules of the table in "Decisions fixed"; 22 under `gleam-double/test/`, the 20
  the ticket counts with `interface_test.gleam` and `rules_test.gleam`; and
  `gleam-double/README.md` and `gleam-double/SPEC-NOTES.md`. It touches nothing under
  `docs/`, `crates/` or `double/`, neither `gleam.toml` nor `manifest.toml`, nor the
  `Justfile` or this ticket.
- The commit after it holds this record, `status: done` and the index's Status cell. The
  ticket's Verification block did not change: it is `verification.sh` from line 8
  (section 8).
- Nothing was fetched. Step 1's `just initialize` and `just gleam-double-deps` were not
  run, by the lead's account; the baseline's `just check` and `just gleam-double` passed
  their checks that the toolchains, the gleam environment and
  `gleam-double/build/packages/` are present.

### 2. Baseline

At `547c34b4dbad59558b88b7fb46ed6e373f01bb35`. Neither file names a commit or says the
tree was untouched. Both were last written a minute after `base.txt`, at 07:12:05Z and
07:12:09Z, before the first red (`ev-red-pin.txt`, 07:19:56Z), and the count, 353, is the
last recorded run's (`ai_tmp/gd/series-review/fix2/ev-gleam-2.txt`) and holds none of the
pin's eight new tests.

`ev-baseline-check.txt`:

```text
$ just check
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

`ev-baseline-gleam.txt`:

```text
$ just gleam-double
...
    Running pachinko_double_test.main
353 tests run, 353 passed
coverage faults: 0
exit status: 0
```

The eight commands of "Files touched", rerun at the base: `ev-counts-base.txt` (07:12:19Z)
and `fences.txt`. Each count is the ticket's. Command 1 prints 105 lines in 14 modules,
with each module's count as the table gives it; command 2, 128 lines; command 3, the 28
lines with `<>`, in `publish`, `settle`, `expiry`, `lease_end`, `core` and `pull`;
command 4, the ten lines and `settle_test.gleam:1230`'s doc comment; command 5, `12` and
`18`; command 6, 1,251 lines in 20 files; command 7, 13 lines in 5 modules; command 8,
23 blocks and 121 lines in 19 blocks, one of which, block 4, holds one line.

### 3. What landed

- **`ids.gleam`**, new: the five open types, each with one constructor wrapping a
  `String`, with the header and doc comments "Decisions fixed" gives. It imports nothing.
- **`gleam-double/src/`.** Each field, parameter and reply of the table in "Decisions
  fixed" takes its id's type; no field, label, argument, function or constructor is
  renamed, reordered, added or removed. `rules.gleam` imports `pachinko_double/ids`
  without a name list and writes `ids.LeaseId` in `LeaseId`, `NeedsLeaseId` and `drive`;
  `core_test.gleam`, `publish_test.gleam` and `rules_test.gleam` write the id's
  constructor `ids.LeaseId`. `config`, `time`, `failure`, `inspection`, `invariants` and
  `internal/decay` do not change. After the change 32 code lines under `src/` name
  `String`, by module as the ticket's table counts them.
- **The defect texts.** Each of the 14 takes the id's `String` out with a plain `let`,
  such as `let ConsumerId(consumer) = pull.consumer`. Where the text is a case arm's or
  an argument's expression, the `let` and the text sit in a block, `{ ... }`, which the
  residue shows (K1, section 6). Every text is the base's: `ev-defect-literals.txt`
  (07:32:47Z) compares each of the six modules' string literals in order, base against
  tree, and prints `same 15/15` for `publish`, `22/22` for `settle`, `13/13` for
  `lease_end`, `9/9` for `expiry`, `11/11` for `pull` and `9/9` for `core`. Verification
  finds no `string.inspect` under `src/` and the ten asserting test lines as they were.
- **The test support.** `test/support/harness.gleam` takes and returns the typed ids as
  the harness table gives them; its fake hands out
  `LeaseId("lease-" <> int.to_string(draws + 1))`.
- **The tests.** 20 test modules wrap each id in its constructor around the literal they
  had, import the constructors by name, and type their helpers' ids. Where a test renders
  or sorts an id, a `let` matches its text out; where it computes one, the constructor
  wraps the result. `competing_test.gleam` goes further (departure 1).
- **The pin**, `gleam-double/test/interface_test.gleam`: the eight new tests and four
  changes of "What the pin gains", with the readers `defined_types`, `signatures`,
  `string_lines` and one the ticket does not name, `string_signatures` (departure 5). It
  was written at step 2 and did not change through step 7 (`ev-pin-unchanged.txt`,
  07:41:23Z: `cmp` against `snap/interface_test.step2.gleam`, `exit status: 0`). At the
  checkpoint one control changed (I1, section 7).
- **The words** (step 7). In `gleam-double/README.md`, the 18 blocks, "Where to look",
  whose second row names `ids.gleam` first, "Entities" (departure 6), the lease-id bullet
  with `lease_id: fn() -> LeaseId` and "A custom type" with
  `Pocket(key: PocketKey("pk"), ...)`. The root's header names `ids` among what a caller
  names and counts nine modules. In `gleam-double/SPEC-NOTES.md`, the three sentences of
  G05, G09 and G10, G68, item 9, "The 68 notes follow" and 22 in the heading table.
  After `just gleam-double-format` (`ev-format-step7.txt`, `exit status: 0`),
  `ev-step7-gleam.txt` (07:36:38Z) reads `361 tests run, 361 passed`,
  `coverage faults: 0`, `exit status: 0`, which includes the guide's drift test, and
  `ev-step7-docs.txt` reads `Validated 49 pages and 50 canonical topics.`,
  `exit status: 0`.

The test count: 353 at the base, 361 from step 2 on, the baseline's plus the pin's
eight. The coverage check's constants stay 93, 32 and 35, and every run below that
reaches the tests reports `coverage faults: 0`. All 47 `just gleam-double-format` records end `exit status: 0`.

### 4. The red steps

Every output in this section ran uncommitted, on base
`547c34b4dbad59558b88b7fb46ed6e373f01bb35`, and each record's first line says so. The
lines a wrapper wrote above the run (the note, the `FAIL` list and the two counts) are
the implementer's, read from the run below them.

#### Step 2: red, the pin

`ev-red-pin.txt` (07:19:56Z), after `ev-step2-format.txt` (`exit status: 0`). The three
controls pass; the eight tests the ticket names fail, and no other.

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
controls passing (no FAIL line): "a type is defined by a type line at column 0, ...
root functions naming String: 10 ( start publisher_joins consumer_joins publish pull renew ack nack release lease_view), on 18 lines
code lines under src/ naming String: 128
...
FAIL interface_test.answers imports only ids, model and time: crashed: ...
FAIL interface_test.src/pachinko_double/ holds the nine vocabulary modules and internal/: crashed: ...
FAIL interface_test.ids holds its interface, and nothing marked: crashed: ...
FAIL interface_test.ids imports nothing: crashed: ...
FAIL interface_test.only ids defines a type named like an id: crashed: ...
FAIL interface_test.no pub fn of the root takes or returns a String: crashed: ...
FAIL interface_test.the lines under src/ that name String are the 32 that hold no id: crashed: ...
FAIL interface_test.no vocabulary module imports a module under internal/: crashed: ...
361 tests run, 353 passed
coverage faults: 0
...
exit status: 1
```

#### Step 3: green, `ids.gleam`

`ev-green-ids.txt` (07:20:15Z), after `ev-format-step3.txt` (`exit status: 0`). The three
tests the ticket names still fail; the `String` lines are the base's 128 and `ids`' five.

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
step 3: ids.gleam written as the ticket gives it
FAIL lines below: 3
FAIL interface_test.answers imports only ids, model and time
FAIL interface_test.no pub fn of the root takes or returns a String
FAIL interface_test.the lines under src/ that name String are the 32 that hold no id
361 tests run, 358 passed
root functions naming String: 10 ( start publisher_joins ... lease_view), on 18 lines
code lines under src/ naming String: 133
...
exit status: 1
```

#### Step 4: one id at a time

Each red changes one call in `owner_test.gleam`'s `partial_tick_keeps_what_ran` and
stops on the compiler's type mismatch, `exit status: 1`, before any test runs
(`ev-red-<id>.txt`, each after `ev-format-red-<id>.txt`, `exit status: 0`). Each fence
below gives the record's first three lines and its compiler block. In the block, the
gutter-only and blank lines, the caret line under the source and the recipe's
`error: recipe ...` line are left out, each marked `...`, and the path is cut. The base
lines the ticket names (121, 123, 128, and 110 to 119) had moved by each stage's
imports, as it foresaw.

`ev-red-PublisherId.txt` (07:21:30Z):

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
PublisherId stage, red: owner_test.gleam partial_tick_keeps_what_ran passes PublisherId("p") to the root's publisher_joins (base line 121); src/ unchanged
--- the compiler error ---
error: Type mismatch
    ┌─ .../gleam-double/test/owner_test.gleam:122:61
...
122 │   let assert Ok(_) = pachinko_double.publisher_joins(queue, PublisherId("p"))
...
Expected type:
...
    String
...
Found type:
...
    ids.PublisherId
...
exit status: 1
```

`ev-red-ConsumerId.txt` (07:23:44Z):

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
ConsumerId stage, red: owner_test.gleam partial_tick_keeps_what_ran passes ConsumerId(consumer) to the root's consumer_joins (base line 123); src/ as the PublisherId stage left it
--- the compiler error ---
error: Type mismatch
    ┌─ .../gleam-double/test/owner_test.gleam:125:45
...
125 │       pachinko_double.consumer_joins(queue, ConsumerId(consumer))
...
Expected type:
...
    String
...
Found type:
...
    ids.ConsumerId
...
exit status: 1
```

`ev-red-PocketKey.txt` (07:26:07Z):

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
PocketKey stage, red: owner_test.gleam partial_tick_keeps_what_ran passes PocketKey(key) as the pocket key of the root's publish (base line 128); src/ as the ConsumerId stage left it
--- the compiler error ---
error: Type mismatch
    ┌─ .../gleam-double/test/owner_test.gleam:133:9
...
133 │         PocketKey(key),
...
Expected type:
...
    String
...
Found type:
...
    ids.PocketKey
...
exit status: 1
```

`ev-red-RecordKey.txt` (07:28:32Z):

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
RecordKey stage, red: owner_test.gleam partial_tick_keeps_what_ran passes RecordKey(key <> "1") as the record key of the root's publish (base line 128); src/ as the PocketKey stage left it
--- the compiler error ---
error: Type mismatch
    ┌─ .../gleam-double/test/owner_test.gleam:134:9
...
134 │         RecordKey(key <> "1"),
...
Expected type:
...
    String
...
Found type:
...
    ids.RecordKey
...
exit status: 1
```

`ev-red-LeaseId.txt` (07:30:53Z):

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
LeaseId stage, red: owner_test.gleam partial_tick_keeps_what_ran makes its lease-id function (base lines 110 to 115) return a LeaseId, which the root's start takes (base line 119); src/ as the RecordKey stage left it
--- the compiler error ---
error: Type mismatch
    ┌─ .../gleam-double/test/owner_test.gleam:122:57
...
122 │     pachinko_double.start(config, harness.clock(clock), lease_id)
...
Expected type:
...
    fn() -> String
...
Found type:
...
    fn() -> ids.LeaseId
...
exit status: 1
```

Each green (`ev-green-<id>.txt`) ran after the stage's `just gleam-double-format` rounds
(`ev-format-g-*.txt`, `exit status: 0` each) and gives the three counts step 4 asks for.
Each matches the ticket's expectation, which the record's note line states:

| Stage | Record | Root functions naming `String`, on lines | `String` lines under `src/` | Tests | Failing |
| --- | --- | --- | --- | --- | --- |
| `PublisherId` | 07:23:37Z | 9 on 16 | 125 | 361 run, 358 passed | root signatures, `String` lines, `answers` imports |
| `ConsumerId` | 07:25:58Z | 6 on 9 | 96 | 361 run, 359 passed | root signatures, `String` lines |
| `PocketKey` | 07:28:26Z | 6 on 8 | 76 | 361 run, 359 passed | root signatures, `String` lines |
| `RecordKey` | 07:30:46Z | 5 on 5 | 58 | 361 run, 359 passed | root signatures, `String` lines |
| `LeaseId` | 07:32:37Z | 0 on 0 | 32 | 361 run, 361 passed | none |

`ev-green-LeaseId.txt`:

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
LeaseId stage, green: expected 0 functions on 0 lines; no FAIL; 361 = 353 + 8
FAIL lines below: 0
361 tests run, 361 passed
root functions naming String: 0 (), on 0 lines
code lines under src/ naming String: 32
...
coverage faults: 0
exit status: 0
```

**The `let` probe**, in the `ConsumerId` stage: `ev-probe-let.txt` (07:25:53Z). Its
note names the three `let` lines it probes, `pull.gleam:122`, `settle.gleam:309` and
`settle.gleam:377`, and ends:

```text
Verdict: a plain let destructures a single-constructor type with no warning; no fallback.
```

Its run 2 is a run whose `gleam build --warnings-as-errors` passed and whose tests ran:
`361 tests run, 358 passed`, `exit status: 1`. Its three failures are the root's
signatures, the `String` lines, and the guide's drift test; the record does not say
why the drift test failed, and the likely reading is that the stage's blocks were not
yet copied into the guide. It is not the stage's green, which is
`ev-green-ConsumerId.txt`. Its run 1 is the first build after `src/` changed, which
stopped on the tests' type mismatches. The first passing build was not saved
(departure 7).

### 5. The swap

**Step 1**, at `547c34b4dbad59558b88b7fb46ed6e373f01bb35`, with only the scratch
`gleam-double/test/swap_test.gleam` added: `ev-swap-strings.txt` (07:12:30Z), which
names no commit. The swapped call built, and every test passed:

```text
$ just gleam-double
...
FAULT swap_test.gleam is on disk and not listed in main
FAULT swap_test.gleam is on disk and main does not import it
353 tests run, 353 passed
coverage faults: 0
...
error: recipe `gleam-double` failed on line 486 with exit code 1
exit status: 1
```

The run failed on two `FAULT` lines, where the ticket quotes one (departure 2). The
scratch module's text is not recorded.

**Step 5**, uncommitted on base `547c34b4dbad59558b88b7fb46ed6e373f01bb35`:
`ev-swap-typed.txt` (07:33:14Z). The build stops on two type mismatches, one for each
swapped argument, and no test runs:

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
step 5: scratch test/swap_test.gleam returns pachinko_double.renew(owner, LeaseId("lease-1"), ConsumerId("c1")); the file is deleted after the run
--- the compiler errors ---
error: Type mismatch
  ┌─ .../gleam-double/test/swap_test.gleam:6:32
...
6 │   pachinko_double.renew(owner, LeaseId("lease-1"), ConsumerId("c1"))
...
Expected type:
...
    ids.ConsumerId
...
Found type:
...
    ids.LeaseId
...
error: Type mismatch
  ┌─ .../gleam-double/test/swap_test.gleam:6:52
...
Expected type:
...
    ids.LeaseId
...
Found type:
...
    ids.ConsumerId
...
error: recipe `gleam-double` failed on line 485 with exit code 1
--- tests run lines (none expected) ---
0
...
exit status: 1
```

Neither record shows the scratch file's deletion. It is in no commit: the work commit's
39 files do not hold it, and Verification's last test, `git status --short` empty, held
at the work commit.

### 6. The residue

`ai_tmp/gd/GD12/residue.py`, run with `python3 -I`, normalises the base's text and the
tree's as step 6 gives it, with two corrections (departure 3), and runs five controls
first, where step 6 names three. At step 6, `residue-step6.txt` (07:34:33Z) ended
`entries: 85`, `exit status: 0`; `ev-residue-controls.txt` (07:34:30Z) shows the three
controls of then. After step 7 and the checkpoint's B2 fix, `residue.txt` (07:55:22Z):

```text
547c34b4dbad59558b88b7fb46ed6e373f01bb35 with uncommitted changes
$ python3 -I ai_tmp/gd/GD12/residue.py   (rerun after the GD12-1 triage fixed B2: ...
ok   a pair that differs only by wrapping shows no difference
ok   a pair with two ids swapped shows one
ok   a pair with a literal changed shows one
ok   a pair that drops an alias from an import line inside a multi-line string shows one
ok   a pair that moves a space inside a multi-line string shows one
...
entries: 86
exit status: 0
```

The 86 are the step-6 run's 85 and the root's header, which step 7 rewrote. From its
first entry on, the run matches the run before B2's fix line for line
(`ai_tmp/review/GD12-1/residue-before-B2.txt`, by `verdicts.md`'s account).
`residue-classified.txt` (07:55:40Z) classifies each entry:

```text
Counts: K1 49, K2 36, S7 1, other 0. (Before the lead's ruling, entries 57 and 58 were
classified D1, a departure for the lead: K2 34, D1 2.)
```

- **K1, 49 entries**, the first kind: the defect texts' `let` lines under `src/`, in
  `core`, `expiry`, `lease_end`, `publish`, `pull` and `settle`, with the block a case
  arm needs for them and the text naming the bound name where it named the field.
- **K2, 36 entries**, the second kind, under `test/`: a `let` that matches an id's text
  out to render a payload, sort keys or note a lease id in a log, in `competing_test`,
  `decay_test`, `inspection_test`, `lease_endings_test`, `lease_test`, `rules_test` and
  `settle_test`; and a constructor around a computed `String`, in `owner_test`'s
  `RecordKey(key <> "1")` and the harness's fake. Entries 57 and 58, the `list.map` and
  `fn` on the right-hand side of `compete`'s sort assertion, are K2 by the lead's ruling
  (departure 1).
- **S7, 1 entry**, the root's `////` header. It is neither kind step 6 allows; it is the
  header "The root's header follows" asks for, and a comment is one token to the
  normaliser, so the reflowed paragraph is one entry. Read literally, the acceptance
  criterion "Step 6's residue holds only the two kinds it allows" does not hold for this
  entry.

What the residue cannot show: a test that wraps a `String` it keeps, such as
`ConsumerId(caller)` in `competing_test.gleam`, normalises to `caller`, so `consumers()`
keeping `List(String)` leaves no entry (`ev-competing.txt`, item 2; C1).

### 7. Checkpoint record

**The plan review**, before the build. Codex, `gpt-6-astra`, one conformance lens, read
the ticket at `d604e23`, after the rulings, against `02236a1`
(`ai_tmp/review/GD12-plan/`). Its eight findings, C1 to C5 medium and C6 to C8 low, are
all valid by `verdicts.md` (07:10:06Z); none was high. The fixes are commit `547c34b`.

**Checkpoint 1**, step 8: conformance and breakage, before the work commit, over
`git diff -M 547c34b4dbad59558b88b7fb46ed6e373f01bb35` plus the untracked files,
`ids.gleam` read in full. There was no `design` lens.

- **Reviewer and model.** One reviewer subagent per lens started Codex, `gpt-6-astra`,
  in the form `ai_tmp/review/reviewer-subagent.md` gives: `codex exec -m gpt-6-astra -s
  read-only -C "$PWD" -o "$DIR/last-$LENS.md" "$(cat "$DIR/brief-$LENS.txt")" <
  /dev/null >| "$DIR/log-$LENS.txt" 2>&1 &`. The exact command lines are not saved. Each
  `log-<lens>.txt` opens `OpenAI Codex v0.169.1`, `model: gpt-6-astra`,
  `sandbox: read-only`, `reasoning effort: ultra`; the process ids are in
  `pid-<lens>.txt` (07:42:07Z and 07:42:08Z). `codex exec --help` was not rerun; the
  record of 2026-11-23 is `ai_tmp/review/codex-exec-help-2026-11-23.txt`.
- **Exit and recovery.** `last-conformance.md` was last written at 07:48:34Z and
  `last-breakage.md` at 07:49:57Z. Each `findings-<lens>.md` opens
  `Source: last-message`, and the rest of it matches its `last-<lens>.md` (`cmp`, for
  this record). Each log ends with its lens's final message; the processes' exit
  statuses are not recorded.
- **Brief.** `brief-<lens>.txt` is `brief-common.txt` followed by `lens-<lens>.txt`
  (`cmp`, for this record). It names this ticket, the series index, the shared rules,
  §11 and §14, the base and the diff, and sends each lens first to step 6's residue and
  `residue.py`, then to the files of `gleam-double/src/`, then to the pin, the harness,
  the guide and the notes, and only then to the other tests. It gives the read limits,
  the step-8 questions and `verification.sh` as Verification. It asks each lens to
  conform to "Decisions fixed", "What the pin gains", "Non-goals", "Files touched" and
  "Acceptance criteria", and checks the four ids' names and declared types against
  `docs/specs/pachinko.allium`. The brief says what is still to come and gives no
  hand-back conclusion.
- **Triage.** `verdicts.md` (07:59:10Z) gives each verdict, the probe behind it and the
  fixes. C1 and C2 rest on the lead's rulings. Who ran the triage is not recorded beyond
  its title, "GD12-1 triage verdicts".
- **Rounds.** One. No finding was high, so no second round was owed.

| Id | Severity | Verdict | Reason | Applied |
| --- | --- | --- | --- | --- |
| C1 | low | valid, record only | `consumers()`, `every_turn`, `compete`, `race` and `grants` in `competing_test.gleam` carry `String`s and wrap each where it becomes an id; by the lead's ruling, that is the "wraps the result" of "Decisions fixed" | No code change; departure 1 |
| C2 | low | valid, record only | Residue entries 57 and 58 were classified D1, outside the two kinds; by the lead's ruling they are the second kind | `residue-classified.txt` classifies them K2; departure 1 |
| B1 | medium | invalid | `defined_types` misses `opaque type X`, but gleam 1.19.1 refuses a private opaque type even without warnings as errors, so the build stops before the pin runs | None |
| B2 | medium | valid | `residue.py` tokenised each line apart, so a string spanning lines was read as code: an import line or a space inside one could change unseen | The whole text is tokenised as one stream; two controls added; the rerun finds the same 86 entries |
| I1 | none given; raised by the implementer | valid | The signatures control's `"String"` sat in `f`'s body, so "does not name ... a `String` in a string" was never exercised | The string moved into `f`'s signature; its doc comment says so |

The evidence, in `ai_tmp/review/GD12-1/`:

- `ev-B1-probe.txt` (07:54:25Z), a scratch project outside the tree with the pinned
  gleam: `gleam format --check src`, `exit 0`; `gleam build` without
  `--warnings-as-errors`, `error: Private opaque type`, `Only a public type can be
  opaque.`, `exit 1`. Handed back by `verdicts.md`: a fourth prefix for
  `defined_types` would change the reader the ticket fixes at three.
- `ev-B2-red.txt` (07:55:30Z): the two new controls on the old tokeniser print
  `FAIL a pair that drops an alias from an import line inside a multi-line string shows
  one`, `FAIL a pair that moves a space inside a multi-line string shows one`,
  `a control misbehaved`, `exit status: 1`; after the fix all five print `ok`,
  `exit status: 0`. The reviewer's two scenarios find `runner alias dropped: 0` and
  `coverage spacing moved: 0` before the fix and `1` each after. `residue-before-B2.py`
  is the script before the fix.
- `ev-I1-red.txt` (07:56:15Z), with a broken reader that keeps a string's contents: the
  old control lets "a signature runs from its pub fn line to the line that ends with {"
  pass, `361 tests run, 357 passed`; the corrected control fails it, at
  `#(Function, "reads_signatures"), #(Line, 1927)`, `361 tests run, 356 passed`. The
  reader was restored; the diff against `interface_test-before-I1.gleam` is the
  control's lines and its doc comment alone.
- `ev-verif-sync.txt` (07:56:33Z): no fix touched `verification.sh`, so the ticket's
  block was not edited.

The reruns after the fixes, uncommitted on base
`547c34b4dbad59558b88b7fb46ed6e373f01bb35`: `ev-format.txt`, `exit status: 0`;
`ev-final.txt` (07:56:38Z), `361 tests run, 361 passed`, `coverage faults: 0`,
`exit status: 0`; `ev-docs.txt`, `Validated 49 pages and 50 canonical topics.`,
`exit status: 0`; `ev-lint.txt`, `exit status: 0`; and `ev-verif-precommit.txt`
(07:57:08Z), Verification without `git rev-parse HEAD`, `just check` and the
`git status` test, with the untracked `ids.gleam` added to the changed-files probe,
ending `verif-precommit: every check passed`, `exit status: 0`.

### 8. Verification at the work commit

At `fc5fbac1f1527dfe9c0d5b7895120c8fe712826f`, the work commit, on a clean tree:
`ev-verification-at-work.txt`, last written at 08:00:06Z, is
`sh ai_tmp/gd/GD12/verification.sh`. The script is `#!/bin/sh`, four comment lines,
`base=$(cat ai_tmp/gd/GD12/base.txt)` and `git cat-file -e "$base^{commit}"`, followed
from line 8 by this ticket's Verification block verbatim; a `diff` of the script from
line 8 against the block, made for this record, shows nothing. The run was under plain
`sh`, so no probe's values are traced. It exited 0 under `set -eu`, so every `test`
held: `ids.gleam` imports nothing and is the one home of the five types, each open with
one constructor; the root's twelve signatures name no `String`; the code lines naming
`String` are 32, by module as "Decisions fixed" counts them; no `string.inspect`; the
ten asserting test lines as they were; the walkthrough's imports within the witness and
`ids` among them; no id as a `String` in the guide or in the three sentences of the
notes; G68 once, last under its heading, item 9 with its question, 68 notes and 22 in
the heading table, G68 and item 9 word for word; exactly the fifteen files under
`src/` changed, and neither `gleam.toml` nor `manifest.toml`; no `erl_crash.dump`; and
`git status --short` empty after `just check`. The two `273 tests run: 273 passed, 0
skipped` lines inside `just check` are the Rust workspace's tests, not the double's.

```text
$ sh ai_tmp/gd/GD12/verification.sh
fc5fbac1f1527dfe9c0d5b7895120c8fe712826f
...
361 tests run, 361 passed
coverage faults: 0
...
Validated 49 pages and 50 canonical topics.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
exit status: 0
```

### 9. Departures

1. **`competing_test.gleam`'s `compete` assertion and `consumers()`, by the lead's
   ruling on D1** (`ev-competing.txt`; C1 and C2). Both sides of `compete`'s
   `list.sort(..., string.compare)` hold `PocketKey`s after the change, and an id has
   no order. The left side's existing `fn(grant)` takes each key's text out with
   `let PocketKey(key) = grant.pocket_key`; the right side gains a `list.map` with a new
   `fn(pocket_key)` that does the same over `list.take(pocket_keys, granted)`. The texts
   compared are the base's. The lead ruled this the second kind step 6 allows; residue
   entries 57 and 58. `consumers()` stays `List(String)`, and each caller is wrapped
   where it becomes an id: `harness.pull(double, ConsumerId(caller), limit)`,
   `harness.consumer_joins(double, ConsumerId(caller))`,
   `assert consumer == ConsumerId(caller)` and `NoPocketAvailable(ConsumerId(caller))`.
2. **Step 1's swap failed on two `FAULT` lines**, where the ticket quotes one
   (`ev-swap-strings.txt`): `... is on disk and not listed in main` and
   `... is on disk and main does not import it`.
3. **`residue.py` goes past step 6's text.**
   - Two corrections to the normaliser (`ev-residue-correction.txt`): `X(a)` also takes
     a tuple's field, as in `RecordKey(version.0)`, and a `,` before the `>>` that
     closes a bit array is dropped like one before `)`, `]` or `}`. The rules as
     written leave 94 entries; the corrected script 85. The nine it removes are four
     `RecordKey(version.0)` wraps and five commas `gleam format` writes; the
     uncorrected script fails its own first control.
   - After B2, the whole text is tokenised as one stream, so a string that spans lines
     is read as text.
   - It runs five controls, where step 6 names three; the last two are B2's.
   - The residue has a third class, S7, entry 1, the root's header (step 7), beside the
     two kinds. The final counts are K1 49, K2 36, S7 1 (`residue-classified.txt`).
4. **The formatter moved lines the ticket did not name** (`format-diff.txt`, the diff
   of the tree before and after each of 46 `just gleam-double-format` rounds). It
   sorted imports and rewrapped long lines. None of the 32 `String` lines under `src/`
   was split or changed: under `src/` it added or removed 136 lines, 48 of them naming
   `import`, and none of them holds the text `String` (checked for this record with
   `grep -F String` over those lines, which prints nothing; the same lines with a word
   boundary print nothing too, and that pattern counts 1 on `x: String,`). Step 2's
   round, on the pin alone, is not in the file: it holds 46 rounds of the 47 records.
5. **The pin goes past the ticket's text.** It adds a reader the ticket does not name,
   `string_signatures`, which keeps each `pub fn` whose signature names `String`, with
   those lines trimmed. "no pub fn of the root takes or returns a String" asserts that
   the root has twelve signatures and that `string_signatures` finds none, and the
   `String` lines test holds the 32 rows in `string_lines_held`. The header's wording on
   the three readers and the new assertions is the implementer's, since the ticket fixes
   no text for it. The signatures control was corrected after its red at the checkpoint
   (I1).
6. **The guide's new "Entities" sentence cites "(G68)"**, which the ticket's wording
   does not give, and that paragraph, like the root's header paragraph, is reflowed
   around the changed words.
7. **The `let` probe held, with no fallback** (`ev-probe-let.txt`), but its first
   passing build's output was not kept: an identical rerun overwrote it, and the record
   holds that rerun as run 2.

### 10. What was not run, and what was not recorded

Not run:

- A push or a pull request. Both need authorisation (step 9); nothing left the worktree.
- `just initialize`, `just gleam-double-deps` or any other network recipe. The review's
  model calls are the only network use, which the checkpoint authorises.
- A second review round. None was owed.
- A review of the triage's fixes: I1's change to the pin and B2's to `residue.py`.
- The full Verification before the work commit. `ev-verif-precommit.txt` left out
  `git rev-parse HEAD`, `just check` and the `git status` test; section 8's run is the
  first full one.
- Verification under `sh -x`: its probes' values are not traced, only its exit.

Not recorded: the scratch swap module's text at step 1, and its deletion at steps 1 and
5, beyond its absence from the work commit and the clean tree; the `let` probe's first
passing build; that the baseline ran on an untouched tree, beyond the files' times and
count; the exact `codex exec` command lines and the Codex processes' exit statuses; who
ran the triage; and the models of the subagents.

## Open points

- **The test diff is large.** Most of it is mechanical, and step 6's residue is what
  the review reads to see that nothing else moved. Splitting the ticket by id would give
  five smaller diffs, each of which builds; the maintainer chose one ticket, and step 4
  stages it by id within it.
