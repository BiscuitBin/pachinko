---
id: T48
title: "Pocket keys and record keys have a format: non-empty, ASCII a-z, 0-9 and -"
status: todo
depends_on: [T43]
parallel_with: []
branch: ticket/t48-key-format
estimated_size: L
---

# T48: Pocket keys and record keys have a format: non-empty, ASCII `a-z`, `0-9` and `-`

## Context

`docs/specs/pachinko.allium` declares a pocket's key and a record's key each as a
`String` and states no format for either: no length, no character set, nothing about an
empty one. Both implementations and both doubles accept any text. The Rust library and
service (`crates/`) store a key as given, and catalog row S020 pins that a key with
spaces, a slash, non-ASCII characters and a NUL round-trips unchanged. The Gleam double
(`gleam-double/`) wraps each key in a type that checks nothing.

The Gleam double's notes, `gleam-double/SPEC-NOTES.md`, put the gap to the maintainer as
item 9 of "What this double raises that no `N`-note holds", with note G68: do pocket keys
and record keys have a format, such as non-empty? The maintainer ruled on 2026-11-25.
This ticket lands the ruling for the two keys in the specification, the library, the
service, the Python double, the Gleam double, the scenario catalog and the handbook.

Nothing in this ticket re-opens the ruling. A finding that the ruling is wrong goes to
the maintainer as a question, never into a fix.

### The ruling (2026-11-25): a key has a format

- **The format.** A pocket key and a record key each have the same format: at least one
  character, no whitespace, and only the ASCII lowercase letters `a-z`, the digits `0-9`
  and the hyphen `-`.
- **Uppercase is refused.** An uppercase letter is outside the set, so a key holding one
  is refused like any other key outside the format. No implementation folds case,
  trims, or normalises a key into the format; a key is in the format as given, or it is
  refused.
- **Two keys, one format.** The ruling names the pocket key and the record key alike.
- **Not this ticket's.** The same day's rulings on publisher, consumer and lease ids,
  which give them a UUID format, are not this ticket's and are landed apart from it.

### What the ruling leaves open

Step 1 puts each of these to the maintainer. None is assumed, and no step before the
answers chooses one.

1. **A length cap.** None, or a figure. If a figure: whether it is fixed in the module,
   a `config` value with a `serve` flag (the language reference's "Config" section
   writes exactly this shape, `requires: length(password) >= config.min_password_length`),
   or a request-shape limit of the service alone, as `--max-payload-bytes` is. A refusal
   for length must not read as a refusal for capacity, which `NeverRefusedForCapacity`
   rules out for a publish. With no cap a key is bounded only by `--max-body-bytes` and
   by whatever the HTTP stack allows in a path (unverified; step 2 finds out).
2. **The hyphen.** Whether a key may begin or end with `-`, hold `--`, or be `-` alone.
   The ruling's set admits each; ask whether that is intended.
3. **How the module states the format.** "The options for the module" below gives a
   sketch of each: a black-box predicate in `requires`; a value type; and, beside either,
   an invariant over the keys the state holds.
4. **The refusal.** Its name, or names if each key has its own; what it carries; and its
   place in the check order T43 decides. For a publish: before or after the equal-rank
   refusal, and, if T45 has not landed, against `UnknownPublisher`; the pocket key
   before the record key, or one refusal for both. For a batch publish: a whole-request
   refusal before any row reaches the owner, as a payload that is not base64 is, or a
   per-row outcome, as `equal_rank` is; and, if whole-request, its place among the
   batch's other whole-request checks.
5. **Which triggers are checked.** "Where a key comes in" below lists them. A publish
   carries both keys and is checked. An ack and a nack carry a record key: either it is
   checked too, with a refusal placed against the lease refusals and `RecordNotHeld`
   (scenario S070 pins the lease check before the record check), or it is not, and an
   ack of a key outside the format is refused as an ack of any key the lease does not
   hold is today, since no stored record can carry such a key.
6. **Where the check lives.** Three places are possible, and they differ in what moves:
   - the library's key types refuse at construction, so no `PocketKey` or `RecordKey`
     outside the format can exist and a publish cannot carry one; every construction
     site changes shape ("What exists today" counts them);
   - the library's rules refuse, in `Queue::publish` (and in the mark actions if point 5
     says so), and the key types stay as they are;
   - the service alone refuses, at the edge, and the library and the doubles keep
     accepting any text. Then the library no longer does what the module states unless
     the module puts the format outside its scope, as its Excludes put the wire format;
     say so in the question.
   The Python double and the Gleam double follow the library's answer in their own
   terms (a constructor that refuses, or a guard in the Publish table).
7. **The wire form.** The HTTP status and error code. Whether a path segment is checked
   after percent-decoding, so that `%61` is the key `a`, or is refused unless it is
   written in the format as sent (decoding is the transport's, but it is close to the
   normalising the ruling forbids). What a publish with an empty path segment answers
   today is unverified; step 2 finds out and the answer covers it.
8. **S020 and the new rows.** S020, "Payload bytes and keys round-trip unchanged", cannot
   hold for its keys. Its new title; whether its keys become the whole of the set
   (`a-z`, `0-9` and `-`) and its payload half stays; and the titles of the new rows that
   pin the refusal. New rows take the catalog's next free id when the step runs, never
   S094 by assumption.
9. **The 88-byte envelope.** `--max-body-bytes` must be at least one maximum payload plus
   an 88-byte envelope, computed from a batch row with both keys empty, which the
   service will now refuse. Either the figure stays as a lower bound and the text that
   derives it is reworded, or it is recomputed from the shortest row the service
   accepts, one character per key (two bytes more), which moves the usage error's
   threshold.
10. **The base branch.** See the last bullet of "What exists today".

### The options for the module

The facts come from `allium-skill-reference/allium/language-reference.md`; read its
sections "Field types" (its "Primitive types"), "Value types", "Black box functions",
"Preconditions (requires)", "Config" and "Invariants" before step 1.

- "Primitive types have no properties or methods. For domain-specific string types
  (email addresses, URLs), use value types or plain `String` fields with descriptive
  names. For operations on primitives beyond the built-in operators, use black box
  functions (e.g., `length(password)`, `hash(password)`)."
- Black box functions are "pure (no side effects) and deterministic", always called
  free-standing, and their behaviour is "described by comments or deferred
  specifications".
- A `requires` that is not met rejects an external stimulus trigger: "The action is
  rejected; caller receives an error". How that rejection is named is T43's open point
  6.

Each sketch below was written from the reference and has not been run through the
pinned checker. Step 1 runs each and quotes the result beside it. The name
`well_formed_key` is a placeholder, not a decision.

**Form A: a black-box predicate in `requires`.** A comment states the format once, and
each of the five rules that share `Publish(…)` requires it of both keys, so that a
publish with a key outside the format matches none of them:

```text
-- A key is well formed when it holds at least one character and every
-- character is an ASCII lowercase letter a-z, a digit 0-9 or a hyphen -.
-- Decided by the maintainer on 2026-11-25.
rule PublishIntoNewPocket {
    when: Publish(publisher, pocket_key, record_key, rank, payload)
    requires: well_formed_key(pocket_key)
    requires: well_formed_key(record_key)
    requires: not exists Pocket{key: pocket_key}
    ...
}
```

What Form A leaves for the maintainer: the two `requires` repeat in five rules (and in
four mark rules if point 5 checks an ack and a nack); where they sit in each rule
decides which `rule-failure.<Rule>.N` obligation ids `just plan-spec pachinko` renumbers.

**Form B: a value type.** `value Key { text: String }`, with `Pocket.key`, `Record.key`,
`SupersededRecord.key`, `DeadLetter.key` and the triggers' keys typed `Key`. The
reference shows a value type with fields and derived values, and no invariant and no
constructor that can refuse, so a value type names the concept and does not by itself
state the format. Unverified, for step 1: whether the checker accepts an invariant inside
a `value`, and whether lookups such as `Pocket{key: pocket_key}` compare value types.

**Form C: an invariant over the stored keys**, beside Form A or B, never alone, since an
invariant states what the state holds and does not reject a trigger:

```text
invariant KeysWellFormed {
    for p in Pockets: well_formed_key(p.key)
}
```

with the same for `Records`, `SupersededRecords` and `DeadLetters`, or one invariant
each. Unverified, for step 1: whether the checker accepts a black-box call in an
invariant. Each added invariant moves the module's count of 35, which T39 and T45 also
move.

A length cap under Form A or C is one more black-box call, `length(key) <=
config.max_key_length`, or a figure in the comment.

### Where a key comes in

From the module and the service, at commit `e92abc1`:

- **Triggers from a caller that carry a key.** `Publish(publisher, pocket_key,
  record_key, rank, payload)` carries both. `Ack(consumer, lease, record_key)` and
  `Nack(consumer, lease, record_key)` carry a record key.
- **Triggers from a caller that carry none.** `Pull(consumer, limit)`, `Renew(consumer,
  lease)` and `Release(consumer, lease)`, and the joins while T45 has not landed.
- **Chained triggers.** `PocketTouched(pocket_key)`, `Handed(pocket, record_key)` and
  `Freed(lease, record_key, ended_at)` carry keys taken from the state, never from a
  caller.
- **Reads.** The `Consuming` surface's lease view is reached by a consumer and answers
  `lease.pocket.key` and each held `record.key`; no read takes a key. On the wire, the
  lease read is `GET /consumers/{consumer_id}/lease`.
- **The service's routes that take a key.** `PUT /pockets/{pocket_key}/records/{record_key}`
  (both keys in the path), `POST /publishers/{publisher_id}/publish` (both keys in each
  row of the body) and `POST /leases/{lease_id}/ack` and `…/nack` (`record_key` in the
  body).
- If T39 has landed, its operator listing of the shed queues may take a key; step 2
  adds it to this list and step 1's point 5 covers it.

### What exists today

Re-read each at step 2; these were true when the ticket was written. Line numbers are of
commit `e92abc1` on the branch `build-gleam-double`, 2026-11-25.

The specification, `docs/specs/pachinko.allium`:

- The Vocabulary's Pocket, "named by a pocket key" (line 37), and Record, "the one slot
  a record key has in a pocket" (line 38). Neither says what a key may hold.
- `Pocket.key: String` (line 125), `Record.key` (line 178), `SupersededRecord.key`
  (line 231) and `DeadLetter.key` (line 245).
- The Publish comment (lines 318 to 335) and the five rules that share
  `Publish(publisher, pocket_key, record_key, rank, payload)`, at lines 338, 369, 392,
  422 and 439. No rule reads a key's text; each looks a pocket or a record up by it.
- `AckMarksRecord`, `AckRepeats`, `NackMarksRecord` and `NackRepeats` (lines 601 to 653)
  look the record up as `Record{pocket: pocket, key: record_key}`.
- `invariant OnePocketPerKey` (line 1018) and `OneRecordPerKeyPerPocket` (line 1024).
  The module has 35 invariants and 32 rules (`rg -c '^\s*invariant '` and
  `rg -c '^rule '` over the module print 35 and 32).
- The `Publishing` surface provides `Publish(…)` (line 1114) under
  `@guarantee NeverRefusedForCapacity` (line 1116); the `Consuming` surface exposes
  `lease.pocket.key` (line 1158) and `record.key` (line 1161) and provides `Ack` and
  `Nack` (lines 1170 and 1171).
- The Excludes leave "Transport, wire format and client authentication" to decision
  0019 (line 21). `rg -F 'a-z'` over the module finds nothing.

The library, `crates/libpachinko/`. This ticket was written without reading
`crates/libpachinko/src/`; the facts about it come from its callers and its tests, and
step 2 reads the source:

- The service makes keys with `PocketKey::new(…)` and `RecordKey::new(…)` and uses the
  value at once (`crates/pachinko/src/http.rs` lines 538 and 541,
  `crates/pachinko/src/http/batch.rs` lines 74 and 75,
  `crates/pachinko/src/http/leases.rs` lines 80 and 96), so as called today neither
  constructor can fail. T45 places the id types in `src/ids.rs`; where the two key
  types live, and whether anything reads a key's text, step 2 finds out.
- `PublishError` is `#[non_exhaustive]` with `UnknownPublisher` and
  `EqualRank { key, rank }` (the service's match on it, `crates/pachinko/src/http.rs`
  lines 280 to 290). `LeaseActionError` has `RecordNotHeld(RecordKey)` and
  `MarkConflict(RecordKey)` (the same file's tests, lines 1010 to 1015).
- `tests/queue/publish.rs` lines 279 to 299, S020,
  `payload_bytes_and_keys_round_trip_unchanged`: a pocket key and a record key written
  with `\u{…}` escapes for accented, CJK and astral characters and a NUL (line 284).
- The invariant walk's `POCKETS` (`a`, `b`, `c`) and `KEYS` (`k0`, `k1`, `k2`),
  `tests/queue/walk.rs` lines 42 and 43, are in the format, so the walk reaches no key
  refusal today.
- 59 `// obligation: rule-failure.` tags in `crates/libpachinko/tests` and
  `crates/pachinko/tests` name one of the five Publish rules or the four mark rules
  (`rg` over those two trees for the nine rule names). A new `requires` renumbers some
  of them, according to where it sits.
- `benches/workloads.rs` names its keys `pocket-<n>`, `fill-<n>` and `fresh-<n>` (lines
  101, 127, 140 and 187), and `README.md` shows `PocketKey::new("orders")` and
  `RecordKey::new("o-1")` (lines 127 and 133): all in the format.
- In the trees the ticket's writer could read (the service, the library's `tests/`,
  `benches/` and `README.md`, `loadgen/` and `fuzz/`), `PocketKey::new(` or
  `RecordKey::new(` is called at 60 sites. `src/` adds its own, counted at step 2.

The service, `crates/pachinko/`:

- `src/http.rs`: the routes (lines 97 to 107); `PublishBody` (line 396); the publish
  handler (lines 532 to 553); `Refusal::from_publish` (lines 280 to 290). A library
  refusal the service does not map answers 503 `not_ready` and logs an error
  (`unmapped`, lines 260 to 265), so a new library refusal left unmapped is a 503.
- `src/http/batch.rs`: `batch_rows` (line 48) checks, over every row before any row
  reaches the owner, at least one row, at most `--max-batch-rows`, every payload base64,
  then every payload within `--max-payload-bytes`.
- `src/http/leases.rs`: the ack and nack bodies' `record_key` (line 17).
- `src/config.rs` lines 22 to 26: `BATCH_ENVELOPE_BYTES = 88`, documented as the batch
  body around one row "with empty keys", written out as a JSON row whose `pocket_key`
  and `record_key` are both `""`.
- `tests/http.rs` lines 228 to 247, S020 over the wire (driver `round_trip`): a pocket
  key with spaces, a slash and non-ASCII letters, and a record key with spaces, slashes
  and an accented letter (line 234). The test client percent-encodes every byte outside
  the URL's unreserved set (lines 81 to 85 and 121 to 122).
- `docs/reference/http-api.md` line 16: "keys travel in the path, percent-encoded". The
  route table (lines 26 to 31) lists no key refusal.
- `docs/reference/configuration.md` lines 29 to 33 derive the 88 bytes from the same
  empty-keyed row.

The Gleam double, `gleam-double/`:

- `src/pachinko_double/ids.gleam`: its module comment (lines 1 to 5) says the
  specification "states no format, so each type here wraps any `String` and checks
  nothing"; `PocketKey(String)` (line 21) and `RecordKey(String)` (line 27) are open
  types with a public constructor.
- `test/interface_test.gleam` lines 2001 and 2002 pin the five id types' constructor
  shapes, `PocketKey(String)` and `RecordKey(String)` among them.
- `src/pachinko_double/refusals.gleam`: the double's refusals, whose names are the
  double's because the specification names none.
- `test/publish_test.gleam`: S020's port, tagged at line 113, with its keys at lines
  516 and 517, the same escapes as the library's test.
- `PocketKey("…")` and `RecordKey("…")` are written 1,691 times on 1,311 lines across
  `src/`, `test/` and `README.md`. A constructor that refuses changes every one.
- `README.md` line 143: each key "wraps the specification's `String` and checks nothing
  (G68)".
- `SPEC-NOTES.md`: item 9 of "What this double raises that no `N`-note holds" (lines 203
  to 208), note G68 (from line 2591), and the table under "The maintainer's rulings,
  followed" (from line 43).
- `just gleam-double` fails unless every catalog scenario has a tagged test that passed
  (`docs/reference/commands.md`, the `just gleam-double` row). Whether a test tagged
  with an id the catalog does not yet hold also fails is unverified; step 2 reads
  `test/support/coverage.gleam` and says in which commit each new row lands.

The Python double, `double/`. This ticket was written without reading
`double/pachinko_double/` or `double/tests/`; step 2 reads both. What is known:
`double/README.md` publishes as `queue.publish("p", "pk", "k", 1, b"v1")` (line 537),
and T45 records the double's refusals in `double/pachinko_double/refusals.py` and its
model in `double/pachinko_double/model.py`. Whether the double has key types of its
own, where its S020 port is, and how its coverage test reads the catalog, step 2 finds
out.

The catalog and the handbook:

- `docs/reference/testing.md`: "The 93 scenarios" (line 185), thirteen of them `wire`;
  row S020 (line 214), `wire`, naming the library test and the wire test above.
- `docs/reference/api.md` line 91 (`ack` and `nack` take a `RecordKey`);
  `docs/project/terminology.md` lines 25 and 26 (the Pocket and Record rows name their
  keys); `docs/explanation/security-model.md`, which says nothing of a key's length.
- `CHANGELOG.md`: both crates' sections are Unreleased.

Outside the workspace:

- `loadgen/src/workloads.rs` names its pockets `lg<nonce>-p<n>` (lines 61 to 64) and
  its keys `<tag><run>-t<thread>-<n>` with the tags `r` and `f` (lines 237, 316 and
  358), and `loadgen/src/probe.rs` adds `-probe`, `-a`, `-b` and `-check` (lines 64, 65,
  145 and 187), where the nonce is `<pid>-<nanos>` (line 45): all in the format.
- `fuzz/fuzz_targets/smoke.rs` names no key.
- `main` did not hold `double/` or `gleam-double/` when this ticket was written; both
  were on the branch `build-gleam-double`.

### The keys the change must rewrite

The searches under Verification list every key literal outside the format in the forms
the suites write today: a key type's constructor, a JSON `pocket_key` or `record_key`
field, a `/pockets/…/records/…` path, the first string arguments of a `publish`,
`pocket` or `record` helper and the last of an `ack` or `nack`, and a binding whose name
holds `pocket` or `key`. At commit `e92abc1`, over the trees the writer could read, they
print six lines:

| Where | Keys | What it is |
| --- | --- | --- |
| `crates/libpachinko/tests/queue/publish.rs` line 284 | 2 | S020 |
| `crates/pachinko/tests/http.rs` line 234 | 2 | S020 over the wire |
| `gleam-double/test/publish_test.gleam` lines 516 and 517 | 2 | S020's port |
| `crates/pachinko/src/config.rs` line 24, `docs/reference/configuration.md` line 32 | 4, all empty | the 88-byte envelope (open point 9), not a key a caller sends |

`crates/libpachinko/src/`, `double/pachinko_double/` and `double/tests/` were not read;
step 2 counts them. A key built at run time, from a list of strings or a `format!`, is
not a literal these searches see. Where the check lives in the library or a double, the
suite itself is the backstop: once a key outside the format is refused, a test that
passes one by mistake fails. Where it lives at the service edge only, the library's and
the doubles' suites have no such backstop, and step 2 extends the searches to every
helper shape it finds.

A test that pins the refusal, or the answer point 5 states for an ack or a nack it
leaves unchecked, must hold keys outside the format. Each such literal sits on a line
that carries the marker `key-format: refused` in the file's comment syntax,
and the searches skip those lines. No page shows a request with a refused key; a page
states the format in words.

### Other tickets

- **T43** (`tickets/T43-caller-contract.md`) decides the caller contract: each named
  refusal, the order the refusals are checked and how the module states a named
  refusal. It was written before this ruling, so its open points name no key refusal.
  This ticket lands the key refusal in T43's form and does not choose that form.
- **T45** (`tickets/T45-drop-joining.md`) removes the publisher from the `Publish`
  trigger, `UnknownPublisher` and the join routes, and changes the batch route's path.
  It edits the same five rules and the same handlers. Whichever lands second re-reads
  the other's merged text, and the check order of point 4 is put without
  `UnknownPublisher` if T45 has merged.
- **T39** (`tickets/T39-decide-q2-q6.md`) adds the operator listing and requires S001
  to S093 to pass with their tests unchanged when its switch is off. This ticket
  changes S020's tests. Whichever of the two lands second re-reads the other, and both
  take new scenario ids from the catalog's next free id at the time.
- **T47** (`tickets/T47-start-rule-and-config-bounds.md`) adds the start rule's bounds
  on `config`. If point 1 makes a length cap a `config` value, it is one more bound and
  the start rule states it; whichever lands second adds it.
- **The id format** ruled the same day for publisher, consumer and lease ids
  (`tickets/T49-consumer-id-uuid.md`, and an amendment to T43 for the lease id) touches
  the same id modules in the library and the doubles (`ids.gleam` holds all five types).
  Whichever lands second re-reads those files and keeps the other's change.

### Risks (`CONVENTIONS.md` §13)

- **The specification moves first** (`AGENTS.md`, invariant 1). The format and its
  refusal land in the module and pass `just check-specs` and `just analyse-specs` before
  any test or code changes.
- **A dependency that has not decided.** If T43's decision is absent, or gives no form
  for a named refusal, this ticket stops at step 1 and invents no refusal, rule or
  status code.
- **Keys that are now refused.** Every key in a fixture, a test, the catalog, the Gleam
  double's tests, the Python double's tests, the load generator and the handbook's
  examples may fall outside the format: uppercase, `_`, `.`, `/`, `:`, whitespace,
  non-ASCII, or empty. Step 2 runs the inventory searches before any edit, and they
  must match then; at the close they must find nothing but marked lines. The six lines
  above are today's count in the trees that were read; the three trees not read are
  counted at step 2.
- **A gate that cannot fire.** "Refused and nothing changed" is shown by the whole
  snapshot being equal before and after, and by no rule firing and no lease id or clock
  being read, not by the answer alone. Each search that must find nothing has a
  positive control, and a hand-made line proves the marker skips only marked lines. If
  the library refuses, the walk's domain gains one key outside the format, on a marked
  line, so that the refusal is a label the walk must reach.
- **Half a ruling.** The library, the service and both doubles land together. A library
  that refuses and a service that maps the refusal to `unmapped`'s 503, or, where point
  6 puts the check in the library, a double that still accepts, is a defect.
- **A public surface changes.** A constructor that refuses changes `PocketKey::new` and
  `RecordKey::new` and every caller; a new `PublishError` or `LeaseActionError` variant
  is additive on `#[non_exhaustive]` enums. `just semver` reports either; the report is
  quoted and each change is in both Unreleased sections of `CHANGELOG.md`. Each new or
  changed public item keeps `AGENTS.md` invariant 3 and has a doctest.
- **A wire change.** A key a client sent before may now be refused. The load generator's
  keys are in the format; `just test-loadgen` shows it still runs.
- **The small denominator.** The new branch has its tests, or `just coverage` falls
  below 90%.
- **lychee and typos over `tickets/`.** The hand-back notes write paths and test names
  as code spans, never as links, and spell non-ASCII keys by escape, never literally.

## Goal

A pocket key and a record key outside the format, empty or holding any character other
than ASCII `a-z`, `0-9` and `-`, are refused by the service, and in the specification,
the library, the Python double and the Gleam double where step 1's points 3 and 6 put
the check, at the triggers, under the name, in the order and with the wire form step 1
fixed, and the refusal changes nothing. If point 6 puts the check at the service edge
alone, the module states the format as step 1 answered and the library and the doubles
keep accepting any text. A key in the format behaves exactly as before. No
implementation folds case or normalises a key. No test, fixture, catalog row, example
or page uses a key outside the format except on a line marked `key-format: refused` in
a test that pins the refusal, or the answer point 5 states for an unchecked ack or
nack.

## Non-goals

- No format for publisher, consumer or lease ids.
- No case folding, trimming or normalising, anywhere.
- No change to a rank, a payload or the guards and outcomes of the Publish and mark
  rules beyond the format.
- No names or order for the other refusals, which are T43's, and no removal of joining,
  which is T45's.
- No bound on how many keys or records a deployment holds: `NeverRefusedForCapacity`
  stands.
- No edit to finished tickets, decision records' history or released changelog entries.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | The format, in the form step 1 decides; its `requires` on the Publish rules, and on the mark rules if step 1 says so; the refusal in T43's form; the Vocabulary's Pocket and Record; any invariant step 1 adds |
| `crates/libpachinko/src/` (the key types, `error.rs`, `queue/publish.rs`, `queue/settle.rs`, `lib.rs`, as step 2 finds them) | shared, own lines | The check where step 1 puts it, the refusal, the doc comments and doctests |
| `crates/libpachinko/tests/queue/*.rs`, `tests/queue/walk/model.rs`, `benches/workloads.rs`, `README.md` | shared, own lines | S020, the refusal tests, the walk's domain, model and labels, the renumbered obligation tags; the bench and the example only if a constructor changes |
| `crates/pachinko/src/http.rs`, `http/batch.rs`, `http/leases.rs`, `config.rs` | shared, own lines | The refusal's status and code, the batch rows' check, the ack and nack check if step 1 says so, the envelope comment |
| `crates/pachinko/tests/http.rs`, `tests/batch_publish.rs` | shared, own lines | S020 over the wire and the refusal over the wire |
| `double/pachinko_double/*.py`, `double/tests/*.py`, `double/README.md` | shared, own lines, if the base holds them | The format, the refusal, S020's port and the new tests |
| `gleam-double/src/pachinko_double/ids.gleam`, `refusals.gleam`, `internal/*.gleam` | shared, own lines, if the base holds them | The key types or the guard, the refusal, the module comments |
| `gleam-double/test/*.gleam`, `gleam-double/README.md`, `gleam-double/SPEC-NOTES.md` | shared, own lines, if the base holds them | S020's port, the new tests, the interface pin; the guide's sentence; item 9, G68 and the rulings table |
| `docs/reference/testing.md` | shared, own lines | Row S020, the new rows, the counts |
| `docs/reference/http-api.md`, `api.md`, `configuration.md`, `docs/project/terminology.md`, `docs/explanation/security-model.md` | shared, own lines | The format, the refusal and its code, the path sentence, the envelope's derivation, the two terms, a length cap if step 1 sets one |
| `CHANGELOG.md` | shared, own lines | Both Unreleased sections |
| `tickets/README.md` | shared, own lines | T48's Status cell |
| `tickets/T48-key-format.md` | ticket | `status:` and hand-back notes |

## Steps

0. **The worktree and the baseline.** Create the branch `ticket/t48-key-format` and its
   worktree from `main`, after T43 has merged (`CONVENTIONS.md` §11). Run
   `just initialize` (network, authorised by this step), and `just gleam-double-deps`
   (network, authorised by this step) if the base holds `gleam-double/`. Run
   `just check` before any edit and quote its last line as the baseline, and
   `just gleam-double` and `just python-double` if the base holds the doubles, quoting
   each one's last lines. A red baseline is a `main` defect, not this ticket's work.
1. **Authorisation required: T43's decision and what the ruling leaves open.** T43
   explores and decides; it lands no specification text itself. What it merges is a
   decision record under `docs/decisions/` that carries the checked draft, the
   maintainer's ruling quoted in `tickets/T43-caller-contract.md`'s hand-back notes, and
   the follow-up tickets that land the caller contract. Find all three. If
   `tickets/T43-caller-contract.md` is not `status: done` on `main`, or the decision
   gives no form for a named refusal, stop and tell the maintainer; do not proceed on a
   guess. Otherwise quote the decision's form and order, with its path and commit. If a
   follow-up ticket of T43 lands that form, ask whether this ticket waits for it or
   lands its refusal in the form the record drafts.

   Then, for each of Forms A, B and C (C beside A), write the specification diff, apply
   it to the module, run `just check-specs` and `just analyse-specs`, keep the diff and
   both outputs in the gitignored scratch space `AGENTS.md` names, and put the module
   back with `git restore docs/specs/pachinko.allium`, which this step authorises for
   these trial edits only. Ask the maintainer, quoting this ticket's Context, each
   form's diff and its checker output, the ten points of "What the ruling leaves open",
   in order.

   **Adversarial review** (`CONVENTIONS.md` §14), before asking: the three diffs and the
   question list. Does each form state the whole format and nothing the ruling did not
   say? Does any diff or question decide a point the list says is open, or fold, trim
   or normalise a key? Is a checker result quoted for each form? Is every trigger that
   takes a key from a caller in point 5?

   Then stop until the maintainer answers, and quote the answers in the hand-back,
   dated. An answer that names something outside these points is a new design: record
   it and stop again.
2. **The plan and the inventory.** Use the `plan-change` skill. Read what merged since
   this ticket was written: T43's record and follow-ups, and T39, T45 and T47 if any has
   landed, noting for each whether it changed a Publish or mark rule, a handler named
   here, the catalog's next free id or the invariant count. Re-read everything "What
   exists today" names and correct each line number; read `crates/libpachinko/src/`,
   `double/pachinko_double/` and `double/tests/`, and settle each fact marked unverified.
   Run the inventory searches under Verification before any edit: they must match, and
   their output is quoted as the list of keys to rewrite, with the count per tree. Read
   every helper that takes a key in each suite and every list of keys a test builds,
   and add a pattern for any shape the searches miss, with its own hand-made control.
   Run `just plan-spec pachinko` and keep its output. Write down, before any edit: each
   clause that moves; each test that changes and the clause that moves it; each test
   added; each scenario changed or added; each public item changed; each page to change.
3. **The specification.** Use the `spec-change` skill. Land the format in the form step
   1 fixed, with a comment that spells the set as the ruling does, `a-z`, `0-9` and
   `-`, and dates it 2026-11-25; the `requires` on each trigger step 1 named and the
   refusal in T43's form, where the form step 1 fixed has the rules check the format;
   and the Vocabulary's Pocket and Record, which say a key has
   that format. Run `just check-specs` and `just analyse-specs`, then
   `just plan-spec pachinko`, and list the obligations that appeared or changed id; a
   new `requires` renumbers `rule-failure.<Rule>.N`, and the tags in `crates/` and in the
   doubles follow.

   **Adversarial review** (`CONVENTIONS.md` §14): the specification diff against the
   ruling and step 1's answers. Where the form step 1 fixed has the rules check the
   format, can a publish with a key outside the format reach any rule that creates or
   changes an entity? Where it does not, does the module say so as step 1 answered? Is a
   key in the format answered by exactly
   the rule it was before? Does any text fold, trim or normalise a key?
4. **The library.** Use the `rust-change` skill. Tests first: S020 as step 1 fixed it;
   every character of the set accepted, in one key where points 1 and 2 allow it, or
   else across keys each within point 1's cap and with `-` where point 2 allows it; and,
   where point 6 puts the check in the library, for each key a refusal test over the
   empty key, an uppercase letter, `_`, `.`, `/`, `:`, a space, a tab, a non-ASCII
   letter and a NUL, each on a marked line, each showing the whole snapshot equal before
   and after and no lease id or clock read, and the refusal step 1 named; the hyphen
   cases of point 2 and the length cap of point 1, as answered; an ack and a nack as
   point 5 says, each key outside the format on a marked line. If point 6 puts the
   check at the service edge alone, the library gains no refusal test and its key types
   and rules stay as they are; the refusal tests are step 5's. Show each test of a
   behaviour the unchanged code lacks failing against it and quote the failures; a test
   of a behaviour that already holds, such as S020 with keys in the format, passes
   before and after, and its passing run is quoted as its positive control. Then the
   code, where step 1 put the check, before anything is
   written or read from the clock. Update the walk's domain, model and labels as the
   Risks say. Run
   `just fmt-check`, `just clippy`, `just test`, `just coverage`, `just doc`,
   `just effects` and `just semver` as the change needs them.
5. **The service.** The refusal's status and code as step 1 fixed them, in the single
   publish, each batch row as point 4 says, and the ack and nack bodies if point 5 says
   so; never `unmapped`'s 503. S020 over the wire with keys in the format, which passes
   before and after; a wire test for the refusal on each route point 5 checks, red
   first; and, for an ack or a nack point 5 leaves unchecked, a wire test that a key
   outside the format, on a marked line, gets the answer point 5 states, never 503. The
   envelope in
   `src/config.rs` as point 9 says; whichever way, no comment or page shows a JSON row
   with an empty key as a row the service takes. `just test-loadgen` runs.
6. **The Python double**, if the base holds it: the check, refusal and tests in its own
   terms where point 6 puts the check in the library (none if it puts it at the service
   edge alone), S020's port, and its README, with `just python-double` green. In the
   notes, say only what is now true; cite notes by id.
7. **The Gleam double**, if the base holds it: the key types or the guard as step 1
   fixed and the refusal in `refusals.gleam`, where point 6 puts the check in the
   library (neither if it puts it at the service edge alone), S020's port, the new
   tests, the interface pin
   in `test/interface_test.gleam`, the module comment of `ids.gleam` and the README's
   sentence (each says what the key types check, and still says of the three ids
   whatever is true of them when this ticket lands), with `just gleam-double` green.
   In `gleam-double/SPEC-NOTES.md`, item 9 and G68 say the keys' half is ruled and
   landed, and the rulings table gains a row for it.
8. **The scenario catalog.** In `docs/reference/testing.md`, row S020 carries its new
   title and test names, each new row its id, title, tags and tests, and the page's
   counts match. Each new id is tagged once per suite. Land each row with the first
   suite that tags it, if step 2 found that a tag ahead of its row fails.
9. **The handbook.** Use the `review-docs` skill for every page in Files touched:
   `docs/reference/http-api.md` states the format, the refusal on each route point 5
   checks and its code, what an unchecked ack or nack answers if point 5 leaves one, and
   replaces "keys travel in the path, percent-encoded" with what
   point 7 decided; `docs/reference/configuration.md` derives the envelope as point 9
   says; `docs/reference/api.md` and `docs/project/terminology.md` state the format
   where they name a key; `docs/explanation/security-model.md` says what bounds a key's
   length. Record the change in both Unreleased sections of `CHANGELOG.md`. Run
   `just check-docs`.
10. **Close.** Set `status: done` here and in `tickets/README.md`, and write the
    hand-back notes, each quoted output headed by its commit.

    **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Can the
    gate pass while a key outside the format is still accepted over HTTP, or in the
    library or either double where point 6 put the check there? Is any unmarked key
    outside the format left, or a marked line outside a test that pins the refusal or
    point 5's answer for an unchecked ack or nack? Then run `just check`.

    **Authorisation required:** pushing the branch and opening the pull request are each
    asked for separately.

## Acceptance criteria

- T43's decision and the maintainer's answers at step 1 are quoted in the hand-back
  with their dates, and nothing before them chose a form or an answer.
- The module states the format once, spelt `a-z`, `0-9` and `-`, dated 2026-11-25, in
  the form step 1 fixed; where that form has the rules check it, every trigger step 1
  named requires it of each key it carries and the refusal has the name and the place
  step 1 fixed; where point 6 put the check at the service edge alone, the module says
  what step 1 answered of its scope. `just check-specs` and `just analyse-specs` pass,
  and every `// obligation:` id in `crates/` is in `just plan-spec pachinko`'s output or
  names a `@guarantee`.
- Over HTTP, and in the library and both doubles where point 6 put the check there, a
  key outside the format (each of the cases of step 4) is refused with that name, the
  snapshot is equal before and after, and no lease id or clock is read; each case is
  pinned by a test that was red before the code changed, and the red output is quoted.
- Every character of the set is accepted and round-trips unchanged, in the library,
  over HTTP and in both doubles: in one key where points 1 and 2 allow it, or else
  across keys each within point 1's cap and with `-` where point 2 allows it.
- Over HTTP, each route point 5 checks answers the status and code step 1 fixed, and
  never 503 `not_ready`, for a key outside the format; an ack or a nack point 5 leaves
  unchecked answers what point 5 states, never 503; a batch with one such row lands
  what point 4 says.
- The inventory searches matched at step 2, with their output quoted, and print nothing
  at the close; their positive controls pass; every line marked `key-format: refused`
  is in a test that pins the refusal, or the answer point 5 states for an unchecked ack
  or nack, and the hand-back lists them.
- The invariant walk reaches the key refusal and is required to, if the library refuses.
- Row S020 and the new rows of `docs/reference/testing.md` name tests that exist, and
  the page's counts match the suites.
- `gleam-double/SPEC-NOTES.md`'s item 9 and G68 say what landed.
- `just semver`'s report is quoted, both Unreleased sections of `CHANGELOG.md` record the
  change, every new or changed public item keeps invariant 3 and has a doctest,
  `just coverage` reports at or above 90% and `just effects` passes.
- `just python-double` prints `Ran <n> tests` and ends `OK`; `just gleam-double` ends
  with `<n> tests run, <n> passed` and `coverage faults: 0`; each exits 0, where the
  base holds the double.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
search that must find nothing passes only on `rg`'s status 1, never on an error, and
follows a positive control over the same files. No command feeds a pipe. Drop from
`trees` any path the base does not hold, and say so; a missing path makes `rg` exit 2,
which is a failure, not a pass. The tickets, `CHANGELOG.md` and the two
`SPEC-NOTES.md` files are not searched: they record what was asked and decided. At step
2 the key search must match: `rg` exits 0, the test after it prints "exit 1", and the
output is the inventory. At the close it must find nothing: the test prints "exit 0".

```sh
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
# the format is stated in the module: at step 2 this exits 1, at the close 0
rg -n -F 'a-z' docs/specs/pachinko.allium; echo "exit $?"
trees='crates loadgen/src fuzz README.md docs gleam-double/src gleam-double/test gleam-double/README.md double/pachinko_double double/tests double/README.md'
# a string literal that is empty or holds a character outside a-z, 0-9 and -
bad='"([^"]*[^a-z0-9"-][^"]*)?"'
# lines that carry the marker are skipped
keep='^(?!.*key-format: refused)'
s1="(PocketKey|RecordKey)(::new)?\\($bad"
s2="\"(pocket_key|record_key)\" *: *$bad"
# a path segment that holds a character outside the set, or is empty
s3="/pockets/[a-z0-9-]*[^a-z0-9{/ \"\`-]|/records/[a-z0-9-]*[^a-z0-9{/ \"\`\\\\?-]|/pockets/(?=[/ \"\`]|\$)|/records/(?=[ \"\`\\\\?]|\$)"
s4="\\.(publish|pocket|record)\\((\"[a-z0-9-]+\", *){0,2}$bad|\\.(ack|nack)\\([^)]*, *$bad *\\)"
s5="(pocket|key)[a-z_0-9]*(\\) *= *\\(\"[^\"]*\", *$bad|\\)? *= *\\(?$bad)"
keys="$keep.*($s1|$s2|$s3|$s4|$s5)"
# hand-made control: prints lines 1 to 12 and none of 13 (marked), 14 (in the format)
# and 15 (a route template)
ctl=$(mktemp)
printf '%s\n' \
  'PocketKey::new("Tenant_A")' \
  'RecordKey("")' \
  '{"pocket_key": "a b", "record_key": "k"}' \
  'PUT /pockets/Tenant/records/k1' \
  'fixture.publish("a", "K1", 1, "x")' \
  'queue.publish("p", "pk", "k.1", 1, b"v")' \
  'queue.ack(&c, lease, "k/1")' \
  'let (pocket, key) = ("a", "key-\u{e9}")' \
  'let key = "";' \
  'client.put("/pockets//records/k", b"v")' \
  'client.put("/pockets/a/records/", b"v")' \
  'PUT /pockets/a/records/' \
  'RecordKey::new("UPPER") // key-format: refused' \
  'PocketKey::new("tenant-a"), RecordKey("k-1"), "/pockets/tenant-a/records/k1"' \
  'PUT /pockets/{pocket_key}/records/{record_key}' >| "$ctl"
rg -n --pcre2 "$keys" "$ctl"; echo "exit $?"
rm -f "$ctl"
# positive controls: each form is found in the trees (each exit 0)
rg -q '(PocketKey|RecordKey)(::new)?\("' $trees; echo "exit $?"
rg -q '"(pocket_key|record_key)" *: *"' $trees; echo "exit $?"
rg -q '/pockets/[^/]*/records/' $trees; echo "exit $?"
rg -q '\.(publish|pocket|record|ack|nack)\(' $trees; echo "exit $?"
# no key outside the format is left: at the close rg must exit 1, so this prints "exit 0"
rg -n --pcre2 "$keys" $trees
[ "$?" -eq 1 ]; echo "exit $?"
# the refusal tests exist and are marked (exit 0); list each line in the hand-back
rg -n 'key-format: refused' $trees; echo "exit $?"
just plan-spec pachinko; echo "exit $?"
just test; echo "exit $?"
just coverage; echo "exit $?"
just effects; echo "exit $?"
just semver; echo "exit $?"
just test-loadgen; echo "exit $?"
just check-docs; echo "exit $?"
# only if the base holds the double: prints "Ran <n> tests", ends "OK"
just python-double; echo "exit $?"
# only if the base holds the double: ends "<n> tests run, <n> passed", "coverage faults: 0"
just gleam-double; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

When this ticket was written, at commit `e92abc1` and over the trees its writer could
read (without `crates/libpachinko/src`, `double/pachinko_double` and `double/tests`),
the control printed lines 1 to 12 and exited 0, each positive control exited 0, the
marker search exited 1, and the key search exited 0 with the six lines of "The keys the
change must rewrite". The empty-segment alternatives of `s3` also match a path built by
joining, such as `"/pockets/" + key`; none was in those trees, and step 2 narrows the
pattern, with a control line, if the trees it reads hold one.

## Hand-back notes

### Baseline

### T43's decision and the maintainer's answers (step 1)

### The merged state and the inventory (step 2)

### The specification

### Red tests

### Verification output

### Adversarial reviews

### Handed back

## Open points

- The ten points of "What the ruling leaves open" are step 1's to settle, after T43's
  decision.
- The checker's verdict on each form was not known when the ticket was written; step 1
  produces it.
- `crates/libpachinko/src/`, `double/pachinko_double/` and `double/tests/` were not read
  when the ticket was written: where the key types live, whether anything reads a key's
  text, the Python double's key types and S020 port, and the count of keys outside the
  format there are step 2's.
- Whether a test tagged ahead of its catalog row fails either double's coverage check is
  step 2's to find out.
- What the HTTP stack answers for an empty path segment, and how long a path it admits,
  are step 2's to find out.
- Whether the doubles move in this ticket depends on the base step 0 finds.
