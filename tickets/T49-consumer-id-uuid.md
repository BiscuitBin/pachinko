---
id: T49
title: "A consumer id is a UUID: any version of the RFC 9562 variant, 36 characters in, lowercase out, held as 16 bytes"
status: todo
depends_on: [T43, T45]
parallel_with: []
branch: ticket/t49-consumer-id-uuid
estimated_size: L
---

# T49: A consumer id is a UUID: any version of the RFC 9562 variant, 36 characters in, lowercase out, held as 16 bytes

## Context

`docs/specs/pachinko.allium` declares a consumer's id as a `String` and states no format
for it: `entity Consumer { id: String }`, which every consumer trigger reaches. Both
implementations and both doubles accept any text. The Rust library and service
(`crates/`) build a `ConsumerId` from whatever the path or the body carries, the Gleam
double (`gleam-double/`) wraps it in a type that checks nothing, and the Python double's
guide (`double/README.md`) passes a plain string. The tests name their consumers `c1`,
`c2`, `nobody` and the like.

The Gleam double's notes, `gleam-double/SPEC-NOTES.md`, put the gap to the maintainer as
item 9 of "What this double raises that no `N`-note holds", with note G68: do publisher
ids, consumer ids, pocket keys, record keys and lease ids have a format? The maintainer
ruled on 2026-11-25. The keys' half is `tickets/T48-key-format.md`'s. The lease id's text
form is an amendment to `tickets/T43-caller-contract.md`, dated 2026-11-25, because T43
puts that form to the maintainer and its follow-up lands it. This ticket lands the
ruling for the consumer id, and for the publisher id if T45 keeps one, in the
specification, the library, the service, the Python double, the Gleam double, the
scenario catalog and the handbook.

Nothing in this ticket re-opens the ruling. A finding that the ruling is wrong goes to
the maintainer as a question, never into a fix.

### The ruling (2026-11-25): the three ids are UUIDs

| Id | Ruling |
| --- | --- |
| Consumer id | A valid UUID. Version 7 is recommended in the handbook, not required. |
| Lease id | A valid UUID. The service always mints a version 7. A lease id a caller presents gets no version check: an id the service never minted is simply not found. |
| Publisher id | A valid UUID, version 7 recommended, not required. Moot if T45 leaves no publisher id. |

What "a valid UUID" means, for all three:

- **The variant.** The RFC 9562 variant, bits `10`: the first hex digit of octet 8 (the
  first digit of the fourth group) is `8`, `9`, `a` or `b`. Any version nibble is
  accepted. So the nil UUID (all zeros) and the max UUID (all `f`) are refused, and
  versions 9 to 15, which RFC 9562 reserves, stay open.
- **The text accepted.** Exactly 36 characters in the 8-4-4-4-12 form, hex digits in
  any case. Braces, the `urn:uuid:` prefix, the 32-digit form without hyphens,
  whitespace and stray hyphens are refused.
- **Held and emitted.** The text is parsed to the 16-byte value at the edge, is always
  emitted as lowercase canonical text, and is never compared as the raw input. Two
  spellings of one UUID that differ only in case name one consumer.
- **Not ruled, and open.** RFC 9562 §8 says UUIDs must not be used as security
  capabilities. If holding a lease id ever grants authority, a version 7's timestamp
  bits make it easier to guess than a version 4. The T43 amendment puts that to the
  maintainer; this ticket gives a consumer id no authority and says so.

Lowercase out is the ruling, not RFC 9562's: RFC 9562 says only that any case is valid
on input. RFC 4122 §3, which it obsoletes, said to output lowercase.

### The sources

Each fact below was gathered from its primary source on 2026-11-25. Only two were
checked a second time when this ticket was written: Python 3.14's `uuid.uuid7()` and
the Go 1.27 standard library's `uuid` package. Every other row, every parser's leniency
and every binary-format fact is unverified until step 1 re-reads it against its source
(network, authorised by that step), except the generator rows, which step 9 re-reads on
the day it writes the handbook's table. URLs are written as code spans.

**RFC 9562** (May 2024, obsoletes RFC 4122):

- §4, the text form (`https://www.rfc-editor.org/rfc/rfc9562.html#section-4`): "any
  given UUID should be represented by the "hex-and-dash" string format"; the ABNF is
  `UUID = 4hexOctet "-" 2hexOctet "-" 2hexOctet "-" 2hexOctet "-" 6hexOctet`, and "the
  alphabetic characters may be all uppercase, all lowercase, or mixed case". The URN
  form appears as a figure; braces and the 32-digit form are not defined. The binary
  form is big-endian, "network byte order".
- §4.1 and §4.2: the variant is in the most significant bits of octet 8, and the version
  in "the most significant 4 bits of octet 6". Table 2 lists version 0 as "Unused." and
  versions 9 to 15 as "Reserved for future definition."
- §5.7 (`https://www.rfc-editor.org/rfc/rfc9562.html#section-5.7`): a version 7 is a
  48-bit big-endian `unix_ts_ms`, the 4-bit version, 12 bits `rand_a`, the 2-bit variant
  and 62 bits `rand_b`; "Implementations SHOULD utilize UUIDv7 instead of UUIDv1 and
  UUIDv6 if possible". No section read ranks version 7 above version 4.
- §5.9 and §5.10 place the nil and the max UUID outside the RFC 9562 variant, which is
  why the variant rule refuses both.
- §6.12, opacity (`https://www.rfc-editor.org/rfc/rfc9562.html#section-6.12`): "avoiding
  parsing UUID values unnecessarily is recommended; instead, treat UUIDs as opaquely as
  possible".
- §6.13, storage (`https://www.rfc-editor.org/rfc/rfc9562.html#section-6.13`): "UUIDs
  SHOULD be stored within database applications as the underlying 128-bit binary value".
- §8, security (`https://www.rfc-editor.org/rfc/rfc9562.html#section-8`): "Implementations
  SHOULD NOT assume that UUIDs are hard to guess ... they MUST NOT be used as security
  capabilities".
- RFC 4122 §3 (`https://www.rfc-editor.org/rfc/rfc4122#section-3`), obsoleted: the
  hexadecimal values "are output as lower case characters and are case insensitive on
  input".

**Prior art: no version check.** CloudEvents 1.0's `id` is any non-empty string
(`https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md`). AWS SQS's
deduplication and group ids are up to 128 characters, not a UUID type
(`https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_SendMessage.html`).
NATS JetStream's `Nats-Msg-Id` states no format
(`https://docs.nats.io/reference/jetstream/api/headers`). Temporal's `request_id` is a
`string` commented "Typically UUIDv4"
(`https://raw.githubusercontent.com/temporalio/api/master/temporal/api/workflowservice/v1/request_response.proto`).
Stripe's `Idempotency-Key` is any string up to 255 characters
(`https://docs.stripe.com/api/idempotent_requests`). PostgreSQL's `uuid` type stores any
UUID "regardless of the origin and the UUID version"
(`https://www.postgresql.org/docs/current/datatype-uuid.html`).

**Prior art: the two that check a version.** Coinbase CDP's `X-Idempotency-Key` "must be
a valid UUID v4 string" (`https://docs.cdp.coinbase.com/api-reference/v2/idempotency`).
Opik requires version 7 trace and span ids, because it places rows in time partitions by
the id's embedded timestamp, and so also bounds that timestamp to a window
(`https://www.comet.com/docs/opik/self-host/configure/uuid_validation`). Pachinko
partitions nothing by a consumer id's timestamp, and a version-nibble check would not
give Opik's guarantee anyway: a version 7 from a skewed clock passes it.

**A version 7 generator per client language**, for the handbook's table (step 9):

| Language | Generator | Since | Kind | Source |
| --- | --- | --- | --- | --- |
| Python | `uuid.uuid7()` | 3.14.0, 2025-10-07; 3.11 to 3.13 lack it | standard library | `https://docs.python.org/3/library/uuid.html` |
| Rust | `Uuid::now_v7()`, features `std` and `v7` | `uuid` 1.6.0, 2023-11-20; current 1.29.3, 2026-11-17 | crate | `https://docs.rs/uuid/latest/uuid/struct.Uuid.html`, `https://github.com/uuid-rs/uuid/releases/tag/1.6.0` |
| Go | `uuid.NewV7()` | Go 1.27, August 2026 | standard library; no `Version()` accessor | `https://go.dev/doc/go1.27`, `https://pkg.go.dev/uuid` |
| Go | `uuid.NewV7()` in `github.com/google/uuid` | v1.5.0, 2023-12-12; latest v1.6.0, 2024-01-23 | module | `https://pkg.go.dev/github.com/google/uuid#NewV7` |
| TypeScript, JavaScript | `v7()` in the npm `uuid` package | 10.0.0, 2024-06-07; latest 14.0.3, 2026-10-09 | package; the platform's `crypto.randomUUID()` is version 4 only | `https://github.com/uuidjs/uuid/blob/main/CHANGELOG.md`, `https://developer.mozilla.org/en-US/docs/Web/API/Crypto/randomUUID` |
| Gleam | `uuid.v7()` and `v7_string()` in `youid` | 1.2.0, 2024-06-16; latest 1.6.1, 2026-11-10 | package; `gleam_stdlib` has no UUID module | `https://youid.hexdocs.pm/youid/uuid.html`, `https://raw.githubusercontent.com/lpil/youid/main/CHANGELOG.md` |
| Elixir | `Ecto.UUID.generate(version: 7)`; the default is still 4 | Ecto 3.14.0, 2026-05-19 | package | `https://ecto.hexdocs.pm/Ecto.UUID.html` |
| Erlang | none released: OTP's `stdlib` (29.1.3) names no UUID module, and okeuday `uuid` (hex `uuid_erl`) has `get_v7` on its master branch only, its latest hex release, 2.0.7 of 2023-10-26, lacking it | n/a | n/a | `https://www.erlang.org/doc/apps/stdlib/api-reference.html`, `https://hex.pm/packages/uuid_erl` |

**The parsers accept more than the ruling.** "A valid UUID" cannot mean "whatever the
client's or the implementation's parser accepts":

- Python's `uuid.UUID(s)` strips `urn:` and `uuid:`, strips braces at the ends and removes
  every `-` before it counts 32 hex digits, so it takes braces, the URN, the 32-digit form
  and stray hyphens, and checks no version or variant
  (`https://raw.githubusercontent.com/python/cpython/3.14/Lib/uuid.py`).
- Rust's `uuid` crate, `Uuid::parse_str` and `try_parse`, takes 32, 36, 38 (braced) and
  45 (URN) characters through one parser, and checks no version or variant
  (`https://docs.rs/uuid/latest/src/uuid/parser.rs.html`). Whether the crate offers a
  hyphenated-only parser, and its minimum Rust version at 1.29.x, were not checked.
- Go's standard `uuid.Parse` takes braces, the URN and the 32-digit form
  (`https://pkg.go.dev/uuid`).
- Gleam's `youid`, `from_string`, takes the URN and the 32-digit form and skips `-`
  anywhere while it counts 32 hex digits
  (`https://raw.githubusercontent.com/lpil/youid/main/src/youid/uuid.gleam`).
- Elixir's `Ecto.UUID.cast` also accepts any 16-byte binary
  (`https://raw.githubusercontent.com/elixir-ecto/ecto/master/lib/ecto/uuid.ex`).
- The npm `uuid` package's `validate` accepts only versions 1 to 8 (and nil and max),
  so it refuses version 0 and versions 9 to 15, which the ruling accepts
  (`https://raw.githubusercontent.com/uuidjs/uuid/main/src/regex.ts`).
- JSON Schema 2020-12's `format: "uuid"` is "for plain UUIDs, not UUIDs in URNs", and
  is an annotation unless assertion is turned on
  (`https://json-schema.org/draft/2020-12/json-schema-validation`).

So the service, the Python double and the Gleam double each need their own check of the
36-character 8-4-4-4-12 form and of the variant, before or after any library parse.

**Binary carriage.** CBOR carries a UUID as tag 37, a 16-byte byte string
(`https://www.iana.org/assignments/cbor-tags/cbor-tags.xhtml`). Protobuf has no
well-known UUID type (`https://protobuf.dev/reference/protobuf/google.protobuf/`); 16-byte
`bytes` is a common choice there. Kafka's `UUID` primitive is "encoded using sixteen
bytes in network byte order" (`https://kafka.apache.org/44/design/protocol`). MessagePack
predefines no UUID extension (`https://github.com/msgpack/msgpack/blob/master/spec.md`).
So with the 16-byte value as the domain type and the text only its rendering, a later
binary codec changes the codec alone.

### What the ruling leaves open

Step 1 puts each of these to the maintainer. None is assumed, and no step before the
answers chooses one.

1. **The refusal.** Its name; what it carries (the text as sent, or nothing); and its
   place in the check order T43 decides. For a pull and the lease read: first, since
   nothing else is checked before the consumer. For an ack, a nack, a renew and a
   release: against the path's lease id parse (today 400 `bad_request` before anything
   reaches the owner) and the lease refusals; whether a malformed lease id in the path
   or a malformed consumer id in the body is reported first, when both are.
2. **The publisher id**, if T45's answer to its point 3 keeps one on the publish routes.
   T45 removes the publisher from the module, so a surviving publisher id is a wire
   field the specification does not name and is checked at the service edge alone.
   Whether it shares the consumer id's refusal or has its own; its place against the
   batch's whole-request checks and the equal-rank refusal; and whether the doubles,
   which would then carry no publisher id, have anything to change. If T45 keeps none,
   this point is moot and the hand-back says so.
3. **How the module states "a UUID".** "The options for the module" below sketches each.
   T45's answer to its point 1 (a `Consumer` entity created on first use, a plain id
   value, or an external entity) decides what the statement attaches to, and T43's
   follow-up decides how the lease id is typed; the consumer id should be stated the
   way the lease id is, or the question says why not.
4. **Where the text check lives.** The ruling fixes that the id is held as its 16 bytes.
   Two places are possible for the parse and its check:
   - the library: `ConsumerId` has a parse from text that refuses, as `LeaseId` has
     `ParseLeaseIdError` today, and the service and any other caller call it;
   - the service edge alone: the library's `ConsumerId` is built only from 16 bytes, and
     each caller parses.
   Either way: does the library's constructor from 16 bytes refuse the nil, the max and
   any value outside the `10` variant, since the variant is a property of the bytes and
   not of the text? If it does not, a caller can build a `ConsumerId` the ruling calls
   invalid without any text at all. The Python double and the Gleam double follow the
   library's answer in their own terms.
5. **The wire form.** The HTTP status and error code for a malformed consumer id, in the
   path of a pull and of the lease read and in the body of the four lease routes. Today a
   malformed lease id is 400 `bad_request`. Whether a path segment is checked after
   percent-decoding, so that `%2D` stands for `-`, or is refused unless written as sent;
   decoding is the transport's, but T48's point 7 asks the same of keys and the two
   answers should agree.
6. **A dependency, or none.** For the Rust: the `uuid` crate (with no `v4`, `v7` or other
   generator feature, since libpachinko performs no effect, `AGENTS.md` invariant 2) or a
   hand-written 16-byte type, as `LeaseId` is today. A crate needs the decision-0007 note
   (`AGENTS.md` invariant 9), and `just deny`, `just deps-unused` and a `Cargo.lock` that
   `just lock` (network) writes. For the Gleam double: `youid` or a hand-written parse over
   `gleam_stdlib`; `youid` needs decision 0021 amended (it says `gleam-double/gleam.toml`
   "names four, each as a range, and nothing else is added without amending this
   record"), and
   `gleam-double/manifest.toml` rewritten by `gleam` through `just gleam-double-deps`
   (network). The Python double takes the standard `uuid` module, no dependency. Whatever
   is chosen, each still needs its own 36-character check ("The sources" above). T43's
   follow-up mints version 7 lease ids and may want the same crate with a generator
   feature in the service; the question says so.
7. **The load generator's ids.** `loadgen/` builds its consumer ids at run time
   (`{prefix}-c{thread}` and the probe's `-a`, `-b` and `-check`) and will be refused.
   It must make UUIDs: from a crate (its own decision-0007 note, and `loadgen/Cargo.lock`
   through `just lock`, network) or built by hand from what it already has (its run
   nonce and a counter, with the version and variant bits set).
8. **The scenario rows.** The titles of the new rows that pin the refusal, the
   any-version rule (nil and max refused, versions 0 and 9 to 15 accepted) and one
   consumer under two spellings; and whether any existing row's title changes. New rows
   take the catalog's next free id when the step runs, never an id by assumption.
9. **The base branch.** See the last bullet of "What exists today".

### The options for the module

The facts come from `allium-skill-reference/allium/language-reference.md`; read its
sections "Field types" (its "Primitive types"), "Value types", "Black box functions",
"Preconditions (requires)" and "Config" before step 1.

- "Primitive types have no properties or methods. For domain-specific string types
  (email addresses, URLs), use value types or plain `String` fields with descriptive
  names. For operations on primitives beyond the built-in operators, use black box
  functions (e.g., `length(password)`, `hash(password)`)." Allium 3 has no UUID type and
  no byte type; the module already types `Payload` as `String` for want of one.
- Black box functions are "pure (no side effects) and deterministic", always called
  free-standing, and described by comments.
- A `config` block declares parameters with defaults. The ruling makes nothing
  configurable: a version 7 is recommended, never required, so no switch is added.

Each sketch below was written from the reference and has not been run through the
pinned checker. Step 1 writes each against the module as T45 left it, runs it and quotes
the result. The names are placeholders, not decisions.

**Form A: a black-box predicate in `requires`.** A comment states the rule once, and
each rule a consumer trigger reaches requires it:

```text
-- A consumer id is a UUID: 36 characters, 8-4-4-4-12 hex digits in any
-- case, of the RFC 9562 variant (octet 8 begins 8, 9, a or b), any
-- version. It is the 16-byte value, never the text as sent. Decided by
-- the maintainer on 2026-11-25.
rule PullClaimsFrontPocket {
    when: Pull(consumer, limit)
    requires: valid_uuid(consumer)
    ...
}
```

What Form A leaves for the maintainer: the `requires` repeats in every Pull, mark,
renew and release rule; where it sits decides which `rule-failure.<Rule>.N` obligation
ids `just plan-spec pachinko` renumbers.

**Form B: a value type.** `value ConsumerId { value: String }`, with the consumer's id,
`Lease.consumer` (under T45's Option B) and `DeadLetter.nacked_by` typed by it. The
reference shows value types with fields and derived values and no constructor that can
refuse, so the type names the concept and states the rule only in its comment.
Unverified, for step 1: whether lookups such as `Lease{consumer: consumer, status:
active}` compare value types, and whether a surface can face one.

**Form C: the text form outside the module.** The Vocabulary's Consumer says a consumer
is named by a UUID, compared as its 128-bit value, and the Excludes' line that leaves
"Transport, wire format and client authentication" to decision 0019 gains the text form
and its refusal, as the lease id's 32-digit parse is the transport's today. No rule
changes. Then the module states no refusal, and the library and the doubles do what the
module states only if point 4 puts the check at the edge; say so in the question.

### What exists today

Re-read each at step 2; these were true when the ticket was written. Line numbers are of
commit `e92abc1` on the branch `build-gleam-double`, 2026-11-25. T43 and T45 move many
of them before this ticket starts.

The specification, `docs/specs/pachinko.allium`:

- The Vocabulary's Consumer (line 53) and Publisher (line 55), which say nothing of an id.
- `entity Publisher { id: String }` (lines 111 to 113) and `entity Consumer { id: String`
  (lines 115 and 116); `DeadLetter.nacked_by: Consumer` (line 248) and
  `Lease.consumer: Consumer` (line 259). T45 removes the publisher and decides the
  consumer.
- `ConsumerJoins(consumer_id)` (lines 312 to 316) and `invariant UniqueConsumerId`
  (line 1012), both removed by T45; `OneActiveLeasePerConsumer` (line 1037).
- The triggers that carry a consumer: `Pull(consumer, limit)` (the `when:` lines 512, 528
  and 570), `Ack` and `Nack` (602, 616, 628 and 643), `Renew` (662) and `Release` (669 and
  693). The `Consuming` surface faces `consumer: Consumer` and provides them (lines 1150 to
  1173). `Batch` and `NoPocketAvailable` carry the consumer back.
- The `config` block (line 292) names no id. The Excludes leave the wire format to
  decision 0019 (line 21). `rg -n -i uuid` over the module finds nothing.

The library, `crates/libpachinko/`. This ticket was written without reading
`crates/libpachinko/src/`; the facts about it come from its callers, its tests and T45,
and step 2 reads the source:

- `ConsumerId` is in `src/ids.rs` (T45). It is built with `ConsumerId::new(…)` from a
  `&str` or a `String`, and read with `as_str()`
  (`crates/libpachinko/tests/queue/pull.rs` line 93).
- `LeaseId` wraps a `u128` and is written as 32 lower-case hex digits, with its own
  `ParseLeaseIdError` (`docs/explanation/architecture.md` line 147): the library already
  owns one parsed 128-bit id type.
- T45 records the registry `consumers: BTreeMap<ConsumerId, Option<LeaseId>>` in
  `src/queue.rs` and `Snapshot.consumers` in `src/queue/inspect.rs`. Any ordered map keyed
  by `ConsumerId` orders by bytes once the id is 16 bytes, not by its text.
- The test fixture's helpers take the consumer id as a `&str` first argument:
  `consumer(id)`, `pull`, `lease_view`, `ack`, `nack`, `renew`, `release`
  (`tests/queue/fixture.rs` lines 69 to 192). The walk's `CONSUMERS` are `c0`, `c1` and
  `c2` (`tests/queue/walk.rs` line 41). `benches/workloads.rs` uses `"consumer"` (line 96)
  and `README.md` `ConsumerId::new("c")` (line 124).

The service, `crates/pachinko/`:

- `src/http.rs`: the routes that carry a consumer id in the path,
  `POST /consumers/{consumer_id}/pull` and `GET /consumers/{consumer_id}/lease` (lines 102
  and 103), and the join route (line 96), which T45 removes. The pull handler builds
  `ConsumerId::new(id)` (line 562); `src/http/lease_read.rs` does the same (line 79).
- `src/http/leases.rs`: the four lease routes' bodies carry `consumer_id: String` (lines 16
  and 23) and each handler builds `ConsumerId::new(body.consumer_id)` (lines 79, 95, 110
  and 124). The path's lease id is "exactly 32 lower-case hexadecimal digits, or 400
  `bad_request` before anything reaches the owner" (lines 63 to 69).
- No reply carries a consumer id: the pull reply holds `lease_id`, `pocket_key`,
  `expires_at` and `records` (`src/http.rs` lines 436 to 446). So today the service never
  renders one.
- No manifest in the workspace, `loadgen/` or `gleam-double/` names a `uuid` package
  (`rg uuid` over `Cargo.toml`, `crates/*/Cargo.toml`, `Cargo.lock`,
  `loadgen/Cargo.toml`, `loadgen/Cargo.lock`, `gleam-double/gleam.toml` and
  `gleam-double/manifest.toml` exits 1).

The Gleam double, `gleam-double/`:

- `src/pachinko_double/ids.gleam`: its module comment (lines 1 to 5) says each type
  "wraps any `String` and checks nothing"; `PublisherId(String)` (lines 9 to 11) and
  `ConsumerId(String)` (lines 15 to 17). `LeaseId(String)` is the double's own (G10) and
  T43's follow-up's.
- `test/interface_test.gleam` line 2001 pins `ConsumerId(String)` among the five
  constructor shapes.
- `ConsumerId(` is written 656 times in 25 files of `src/`, `test/` and `README.md`; the
  tests write `ConsumerId("c1")` and the like, and `test/competing_test.gleam` builds `c1`
  to `c<count>` at run time (line 109).
- `gleam.toml` names three packages and one dev package, which decision 0021 records
  (`docs/decisions/0021-gleam-double-toolchain.md` lines 53 and 54); `youid` is not one.
- `README.md` line 143: each id "wraps the specification's `String` and checks nothing
  (G68)". `SPEC-NOTES.md`: item 9 (lines 203 to 208) and note G68 (from line 2591).

The Python double, `double/`. This ticket was written without reading
`double/pachinko_double/` or `double/tests/`; step 2 reads both. What is known: its
README calls `queue.pull("c1", 1)` and `queue.ack("c1", "lease-1", "k")` (lines 552 and
650) and prints leases as `consumer=c1` (line 559 and on), and `Lease.consumer` is a
consumer id (line 220). The workspace's pixi pins Python 3.14 (`pyproject.toml` line 21),
so `uuid.UUID` and `uuid.uuid7()` are both in its standard library.

The catalog and the handbook:

- `docs/reference/testing.md`: "The 93 scenarios" (line 185); no row's title names an
  id's text, so only the tests under the rows change.
- `docs/reference/http-api.md`: the pull and lease-read routes (lines 28 and 29), the four
  lease routes' bodies with `"consumer_id": "c1"` (lines 30 to 33), and the lease routes'
  check order (from line 130). `docs/how-to/test-and-debug.md` lines 89 to 97 and
  `README.md` lines 127 and 133 send `c1` in a path. `docs/project/terminology.md`'s
  Consumer row (line 33). `docs/explanation/security-model.md` lines 117 and 118:
  anyone "can join under any id".
- `CHANGELOG.md`: both crates' sections are Unreleased.

Outside the workspace and the base:

- `loadgen/src/workloads.rs` builds consumer ids as `{prefix}-c{thread}` (line 194) and
  publisher ids as `{prefix}-t{thread}` (line 186); `loadgen/src/probe.rs` adds `-probe`
  (a publisher, line 64), `-a` (line 65), `-check` (line 145) and `-b` (line 187).
  `loadgen/Cargo.toml` takes `ureq` and `serde_json` only.
- `fuzz/fuzz_targets/smoke.rs` names no consumer.
- `main` did not hold `double/` or `gleam-double/` when this ticket was written; both
  were on the branch `build-gleam-double`.

### The ids the change must rewrite

The searches under Verification list every consumer id literal that is not a valid UUID,
in the forms the suites write today: a `ConsumerId` constructor, a JSON `consumer_id`
field, a `/consumers/…` path, the first string argument of a `pull`, `lease`,
`lease_view`, `ack`, `nack`, `renew` or `release` method, of the free helpers
`consumer`, `pull` and `until_no_lease`, and the second of `marks` and
`wait_for_lapse`, a binding whose name holds `consumer`, and the Python double's printed
lease and dead letter (`consumer=`, `nacked_by=`). At commit `e92abc1`, over the trees
the writer could read, they print 1,062 lines:

| Tree | Lines |
| --- | --- |
| `gleam-double/test/` | 605 |
| `gleam-double/README.md` | 23 |
| `crates/libpachinko/tests/` | 287 |
| `crates/libpachinko/benches/`, `crates/libpachinko/README.md` | 1 and 1 |
| `crates/pachinko/src/` (unit tests in `actor.rs` and `http.rs`) | 37 |
| `crates/pachinko/tests/` | 77 |
| `double/README.md` | 22 |
| `docs/` (`reference/http-api.md` 4, `how-to/test-and-debug.md` 3) | 7 |
| `README.md` | 2 |
| `loadgen/src/`, `fuzz/`, `gleam-double/src/` | 0 |

`crates/libpachinko/src/`, `double/pachinko_double/` and `double/tests/` were not read;
step 2 counts them. An id built at run time is not a literal these searches see. The
ones found when the ticket was written:

| Where | What builds it |
| --- | --- |
| `crates/libpachinko/tests/queue/expiry.rs` line 753 | `ConsumerId::new(format!("c{consumer}"))` |
| `crates/pachinko/src/actor.rs` line 1087 | `ConsumerId::new(format!("c{}-{serial}", self.id))` |
| `gleam-double/test/competing_test.gleam` line 109 | `"c" <> int.to_string(n)` |
| `loadgen/src/workloads.rs` line 194, `loadgen/src/probe.rs` lines 65, 145 and 187 | `format!` over the run's prefix (open point 7) |
| `crates/pachinko/tests/http.rs`, 17 lines | `joined(client, &["c1"])` and the like, a list of ids to join; T45 removes the joins, and step 2 reads what replaced them |

Where the library refuses (open point 4), its suites are the backstop for the run-time
ids: once a non-UUID cannot become a `ConsumerId`, a test that builds one fails. Where
the check lives at the service edge alone, the library's suites have no such backstop,
and step 2 extends the searches to every helper shape it finds. The load generator has
no backstop but a run against the service (step 5). A line that holds no consumer id
but matches a pattern is never marked: the pattern is narrowed, with a control line that
shows the narrowing, as `s6` reads only a printed lease or dead letter and not a keyword
argument, and a free `pull` takes no JSON body.

At the close each literal is a UUID literal, or a named constant whose value is one.
A helper that maps an old name such as `"c1"` to a UUID leaves the old name in place and
keeps the search red, which is intended: those names are what the search removes. A
test that pins the refusal must hold ids that are not valid UUIDs; each such literal sits
on a line that carries the marker `id-format: refused` in the file's comment syntax, and
the searches skip those lines. No page shows a request with a refused id; a page states
the rule in words.

### Other tickets

- **T43** (`tickets/T43-caller-contract.md`) decides the caller contract: each named
  refusal, the order they are checked in, how the module states a refusal, and the lease
  id. Its amendment of 2026-11-25 rules the lease id's text form under this ticket's
  "valid UUID" rule. This ticket lands the consumer id's refusal in T43's form and does not
  choose that form. The lease id is not this ticket's.
- **T45** (`tickets/T45-drop-joining.md`) removes the joins, the publisher from the
  module and `UniqueConsumerId`, and decides how a consumer exists without a join and
  what the publish routes look like. Its amendment of 2026-11-25 says a publisher id it
  keeps is a UUID under this ticket. This ticket starts from T45's merged text.
- **T48** (`tickets/T48-key-format.md`) lands the keys' format ruled the same day. It
  edits the same id modules (`ids.gleam` holds all five types), the same handlers and
  the same catalog. Whichever lands second re-reads the other's merged text, keeps its
  change, and takes new scenario ids from the catalog's next free id. Both ask how a path
  segment is decoded (its point 7, this ticket's point 5); the answers should agree.
- **T43's follow-up** for the lease id mints version 7 ids in the service and revises
  decision 0019. If this ticket takes the `uuid` crate, the two share it: libpachinko
  takes it with no generator feature, and the service's minting is the follow-up's.
- **T39** (`tickets/T39-decide-q2-q6.md`) adds the operator listing of the shed queues,
  which may render a dead letter's `nacked_by`: if T39 has landed, it renders the
  lowercase canonical text, and step 2 adds it to the routes this ticket changes.

### Risks (`CONVENTIONS.md` §13)

- **The specification moves first** (`AGENTS.md`, invariant 1). The rule lands in the
  module and passes `just check-specs` and `just analyse-specs` before any test or code
  changes.
- **A dependency that has not decided.** If T43's decision or T45's answers are absent,
  this ticket stops at step 1 and invents no refusal, rule, form or status code.
- **Ids that are now refused.** Every consumer id in a fixture, a test, the catalog, the
  Gleam double's tests, the Python double's tests, the load generator, the bench and the
  handbook's examples is refused: 1,062 lines in the trees that were read, plus the
  run-time ids above, plus the three trees not read, which step 2 counts. Step 2 runs the
  inventory searches before any edit, and they must match then; at the close they must
  find nothing but marked lines.
- **A gate that cannot fire.** "Refused and nothing changed" is shown by the whole
  snapshot being equal before and after, and by no rule firing and no lease id or clock
  being read, not by the answer alone. Each search that must find nothing has a positive
  control, and a hand-made file proves the searches tell a valid UUID from each refused
  form and skip only marked lines.
- **A raw-text comparison left behind.** A map keyed by the text, an equality on
  `as_str()`, or a test that compares an id with a string literal keeps the old meaning
  under a new type. A test that pulls with one spelling of a UUID and acks with another,
  differing only in case, shows one consumer with one lease, in the library, over HTTP
  and in both doubles.
- **Order by bytes.** Any listing ordered by consumer id (a snapshot's consumers, the
  walk's model) changes order when the id becomes 16 bytes; step 2 finds each and says
  whether anything observable depends on it.
- **Half a ruling.** The library, the service and both doubles land together. A library
  that refuses and a service that maps the refusal to `unmapped`'s 503 `not_ready`, or a
  double that still accepts, is a defect.
- **A public surface changes.** `ConsumerId::new(&str)` and `as_str()` change shape or
  go, and every caller with them; a new error type or variant is additive on a
  `#[non_exhaustive]` enum. `just semver` reports it; the report is quoted and each change
  is in both Unreleased sections of `CHANGELOG.md`. Each new or changed public item keeps
  `AGENTS.md` invariant 3 and has a doctest.
- **The effects boundary.** libpachinko performs no effect (`AGENTS.md` invariant 2). A
  `uuid` crate there carries no generator feature, and `just effects` passes.
- **A wire change.** Every client that sends a consumer id that is not a UUID is refused,
  the load generator among them. It moves in the same change as the service.
- **A UUID is not a credential.** RFC 9562 §8. The handbook must not present a consumer
  id as a secret: the service has no authentication, and anyone can still name any
  consumer id (`docs/explanation/security-model.md`).
- **The small denominator.** The parse and its refusal have their tests, or
  `just coverage` falls below 90%.
- **lychee and typos over `tickets/`.** The hand-back notes write paths, test names and
  URLs as code spans, never as links.

## Goal

A consumer id that is not a valid UUID under the ruling, whatever else it is (empty, a
name, the nil or the max UUID, braced, a URN, 32 digits without hyphens, with whitespace
or a stray hyphen), is refused in the specification, the library, the service, the Python
double and the Gleam double, under the name, in the order and with the wire form step 1
fixed, and the refusal changes nothing. A valid UUID of any version is accepted, held as
its 16 bytes, compared by value, and rendered lowercase; two spellings that differ only
in case are one consumer. The publisher id, if T45 kept one, follows the same rule as
step 1 answered. The handbook recommends version 7 and gives each client language's
generator. No test, fixture, catalog row, example or page uses a consumer id that is not
a valid UUID except on a line marked `id-format: refused` in a test that pins the
refusal.

## Non-goals

- No change to the lease id: its text form is T43's amendment and its minting T43's
  follow-up's.
- No requirement of version 7, no version check on a consumer id, no timestamp window,
  and no warning or log line for a version other than 7.
- No format for pocket keys or record keys, which are T48's.
- No authentication, and no claim that a UUID makes a consumer id secret.
- No names or order for the other refusals, which are T43's, and no removal of joining,
  which is T45's.
- No binary codec; the 16-byte value only makes one a rendering change later.
- No edit to finished tickets, decision records' history or released changelog entries.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | The rule, in the form step 1 decides; the Vocabulary's Consumer; the refusal in T43's form; the publisher id only if step 1 puts it in the module |
| `crates/libpachinko/src/` (`ids.rs`, `error.rs`, `lib.rs` and the modules step 2 finds) | shared, own lines | The 16-byte `ConsumerId`, its parse and refusal where step 1 puts them, the doc comments and doctests |
| `crates/libpachinko/Cargo.toml`, `Cargo.lock` | shared, own lines, if step 1 takes a crate | The dependency with its decision-0007 note |
| `crates/libpachinko/tests/queue/*.rs`, `tests/queue/walk/model.rs`, `benches/workloads.rs`, `README.md` | shared, own lines | The ids, the refusal tests, the two-spellings test, the walk's domain and model, renumbered obligation tags |
| `crates/pachinko/src/http.rs`, `http/leases.rs`, `http/lease_read.rs`, `actor.rs`, and `http/batch.rs` if a publisher id stays | shared, own lines | The parse at the edge, the refusal's status and code, the unit tests' ids |
| `crates/pachinko/tests/http.rs`, `tests/batch_publish.rs` | shared, own lines | The ids, and the refusal on each route over the wire |
| `loadgen/src/*.rs`, and `loadgen/Cargo.toml` and `loadgen/Cargo.lock` if step 1 takes a crate | shared, own lines | UUID consumer ids, and publisher ids if one stays |
| `double/pachinko_double/*.py`, `double/tests/*.py`, `double/README.md`, `double/SPEC-NOTES.md` | shared, own lines, if the base holds them | The parse, the refusal, the ids, the new tests, the guide's examples |
| `gleam-double/src/pachinko_double/ids.gleam`, `refusals.gleam`, `internal/*.gleam`, and `gleam.toml` and `manifest.toml` if step 1 takes `youid` | shared, own lines, if the base holds them | The 16-byte id or its guard, the refusal, the module comments |
| `gleam-double/test/*.gleam`, `gleam-double/README.md`, `gleam-double/SPEC-NOTES.md` | shared, own lines, if the base holds them | The ids, the new tests, the interface pin; the guide's sentence; item 9, G68 and the rulings table |
| `docs/decisions/0021-gleam-double-toolchain.md` | shared, own lines, if step 1 takes `youid` | The package's row |
| `docs/reference/testing.md` | shared, own lines | The new rows and the counts |
| `docs/reference/http-api.md`, `api.md`, `docs/project/terminology.md`, `docs/explanation/security-model.md`, `docs/explanation/architecture.md`, `docs/how-to/test-and-debug.md`, `README.md` | shared, own lines | The rule, the refusal and its code, the generator table, the examples' ids, the term, the not-a-credential sentence |
| `CHANGELOG.md` | shared, own lines | Both Unreleased sections |
| `tickets/README.md` | shared, own lines | T49's Status cell |
| `tickets/T49-consumer-id-uuid.md` | ticket | `status:` and hand-back notes |

## Steps

0. **The worktree and the baseline.** Create the branch `ticket/t49-consumer-id-uuid`
   and its worktree from `main`, after T43 and T45 have merged (`CONVENTIONS.md` §11).
   Run `just initialize` (network, authorised by this step), and `just gleam-double-deps`
   (network, authorised by this step) if the base holds `gleam-double/`. Run `just check`
   before any edit and quote its last line as the baseline, and `just gleam-double` and
   `just python-double` if the base holds the doubles, quoting each one's last lines. A
   red baseline is a `main` defect, not this ticket's work.
1. **Authorisation required: T43's decision, T45's answers and what the ruling leaves
   open.** T43 explores and decides; what it merges is a decision record under
   `docs/decisions/` with the checked draft, the maintainer's answers quoted in
   `tickets/T43-caller-contract.md`'s hand-back notes, and the follow-up tickets. Find all
   three, and T45's answers in `tickets/T45-drop-joining.md`'s hand-back notes. If either
   ticket is not `status: done` on `main`, or T43's decision gives no form for a named
   refusal, stop and tell the maintainer; do not proceed on a guess. Otherwise quote T43's
   form and order and T45's answers to its points 1 and 3, each with its path and commit.

   Re-read every source "The sources" marks for this step (network, authorised by this
   step) and quote each fact beside its URL; a fact the source no longer says is a
   finding for the question, not a silent edit. Then, for each of Forms A, B and C, write
   the specification diff against the module as T45 left it, apply it, run
   `just check-specs` and `just analyse-specs`, keep the diff and both outputs in the
   gitignored scratch space `AGENTS.md` names, and put the module back with
   `git restore docs/specs/pachinko.allium`, which this step authorises for these trial
   edits only. Ask the maintainer, quoting this ticket's Context, each form's diff and
   its checker output, and the nine points of "What the ruling leaves open", in order.

   **Adversarial review** (`CONVENTIONS.md` §14), before asking: the three diffs, the
   re-read sources and the question list. Does each form state the whole rule (the
   variant, any version, the 36-character form, any case in, the 16-byte value) and
   nothing the ruling did not say? Does any diff or question require version 7, or
   decide a point the list says is open? Is a checker result quoted for each form? Is
   every trigger and route that carries a consumer id covered by point 1?

   Then stop until the maintainer answers, and quote the answers in the hand-back,
   dated. An answer that names something outside these points is a new design: record
   it and stop again.
2. **The plan and the inventory.** Use the `plan-change` skill. Read what merged since
   this ticket was written: T43's record and follow-ups, T45, and T39 and T48 if either
   has landed, noting for each whether it changed a consumer trigger, a handler named
   here, an id module or the catalog's next free id. Re-read everything "What exists
   today" names and correct each line number; read `crates/libpachinko/src/`,
   `double/pachinko_double/` and `double/tests/`, and settle each fact marked unverified.
   Run the inventory searches under Verification before any edit: they must match, and
   their output is quoted as the list of ids to rewrite, with the count per tree. Read
   every helper that takes a consumer id in each suite and every list of ids a test
   builds, including the run-time ids above, and add a pattern for any shape the
   searches miss, with its own hand-made control line. Find every map, set or listing
   ordered by consumer id. Run `just plan-spec pachinko` and keep its output. Write down,
   before any edit: each clause that moves; each test that changes and the clause that
   moves it; each test added; each scenario added; each public item changed; each page
   to change.
3. **The specification.** Use the `spec-change` skill. Land the rule in the form step 1
   fixed, with a comment that holds the words "consumer id is a UUID" on one line, states
   the variant, any version, the 36-character form and the 16-byte value, and dates the
   ruling 2026-11-25; the refusal in T43's form; and the Vocabulary's Consumer, which says
   what names a consumer. Run `just check-specs` and `just analyse-specs`, then
   `just plan-spec pachinko`, and list the obligations that appeared or changed id; the
   tags in `crates/` and in the doubles follow.

   **Adversarial review** (`CONVENTIONS.md` §14): the specification diff against the
   ruling and step 1's answers. Can a trigger with a consumer id that is not a valid UUID
   reach any rule that creates or changes an entity? Does a valid UUID of version 0, 4,
   7 or 15 reach exactly the rule it did before? Does any text require version 7, or
   compare the id as text?
4. **The library.** Use the `rust-change` skill. Tests first, red: the refusal for each
   form the Goal lists, each on a marked line, each showing the whole snapshot equal
   before and after, no lease id or clock read, and the refusal step 1 named; a valid
   UUID of versions 0, 4, 7 and 15 accepted; the nil and max refused; one consumer under
   two spellings that differ only in case; rendering lowercase; the constructor from 16
   bytes as point 4 answered. Show each failing against the unchanged code and quote the
   failures. Then the code: the 16-byte `ConsumerId`, the parse where step 1 put it, and
   every caller in the library's tests, bench and README with UUID ids. Update the walk's
   domain and model, and give it a refusal label only if the refusal is inside the
   queue's rules. Run `just fmt-check`, `just clippy`, `just test`, `just coverage`,
   `just doc`, `just effects` and `just semver` as the change needs them, and, if a
   crate was taken, `just lock` (network, authorised by step 1's answer), `just deny` and
   `just deps-unused`.
5. **The service and the load generator.** The parse at the edge, on the pull's and the
   lease read's path and on the four lease routes' bodies, with the status and code step
   1 fixed; never `unmapped`'s 503. A wire test for the refusal on each of those routes,
   and the two-spellings test over HTTP, red first. The publisher id as point 2
   answered. The load generator makes UUIDs as point 7 answered; `just test-loadgen`
   runs, and one `just loadgen` run against a local `pachinko serve` completes with no
   error (`docs/reference/commands.md` gives its arguments; its timings are no gate).
6. **The Python double**, if the base holds it: the same parse, with its own 36-character
   and variant check before `uuid.UUID`, the refusal, the ids and the tests in its own
   terms, and its README, with `just python-double` green. In the notes, say only what is
   now true; cite notes by id.
7. **The Gleam double**, if the base holds it: the 16-byte id or the guard as step 1
   fixed, its own 36-character and variant check, the refusal in `refusals.gleam`, the
   ids, the new tests, the interface pin in `test/interface_test.gleam`, the module
   comment of `ids.gleam` and the README's sentence (each says what the consumer id
   checks, and still says of the keys and the lease id whatever is true of them when this
   ticket lands), with `just gleam-double` green. If step 1 took `youid`, amend decision
   0021 and run `just gleam-double-deps` (network, authorised by step 1's answer). In
   `gleam-double/SPEC-NOTES.md`, item 9 and G68 say the consumer id's half is ruled and
   landed, and the rulings table gains a row for it.
8. **The scenario catalog.** In `docs/reference/testing.md`, each new row carries its id,
   title, tags and tests, and the page's counts match. Each new id is tagged once per
   suite. Land each row with the first suite that tags it, if step 2 found that a tag
   ahead of its row fails a double's coverage check.
9. **The handbook.** Use the `review-docs` skill for every page in Files touched.
   `docs/reference/http-api.md` states the rule, the refusal on each route and its code,
   that version 7 is recommended and not required, and the generator table of "The
   sources", each row re-read against its source on the day (network, authorised by this
   step); its examples' ids become UUIDs. `docs/reference/api.md` and
   `docs/project/terminology.md` state the rule where they name a consumer id;
   `docs/explanation/security-model.md` says a UUID consumer id is not a credential
   (RFC 9562 §8); `docs/how-to/test-and-debug.md` and `README.md` send UUIDs. Record the
   change in both Unreleased sections of `CHANGELOG.md`. Run `just check-docs`.
10. **Close.** Set `status: done` here and in `tickets/README.md`, and write the
    hand-back notes, each quoted output headed by its commit.

    **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Can the
    gate pass while a consumer id that is not a valid UUID is still accepted, in the
    library, over HTTP or in either double, or while two spellings of one UUID name two
    consumers? Is any unmarked non-UUID id left, or a marked line outside a test that
    pins the refusal? Then run `just check`.

    **Authorisation required:** pushing the branch and opening the pull request are each
    asked for separately.

## Acceptance criteria

- T43's decision, T45's answers and the maintainer's answers at step 1 are quoted in the
  hand-back with their dates, and nothing before them chose a form or an answer. Each
  source re-read at step 1 is quoted with its URL.
- The module states the rule once, dated 2026-11-25, in the form step 1 fixed; every
  trigger step 1 named requires it; the refusal has the name and the place step 1 fixed.
  `just check-specs` and `just analyse-specs` pass, and every `// obligation:` id in
  `crates/` is in `just plan-spec pachinko`'s output or names a `@guarantee`.
- In the library, the service and both doubles, each refused form of the Goal is
  refused with that name, the snapshot is equal before and after, and no lease id or
  clock is read; each case is pinned by a test that was red before the code changed, and
  the red output is quoted.
- A valid UUID of versions 0, 4, 7 and 15 is accepted; it is held as 16 bytes, renders
  lowercase, and a pull under one spelling and an ack under another that differs only in
  case act on one lease, in the library, over HTTP and in both doubles.
- Over HTTP, each route that carries a consumer id answers the status and code step 1
  fixed, and never 503 `not_ready`, for a malformed one. The publisher id behaves as
  step 1 answered, or the hand-back says T45 left none.
- The inventory searches matched at step 2, with their output quoted, and print nothing
  at the close; their positive controls pass; every line marked `id-format: refused` is
  in a test that pins the refusal, and the hand-back lists them.
- The load generator's ids are UUIDs, `just test-loadgen` passes, and one `just loadgen`
  run completed with no error.
- Any dependency taken has its decision-0007 note (and, for the Gleam double, its row in
  decision 0021), and `just deny` and `just deps-unused` pass.
- `docs/reference/http-api.md` holds the rule, the refusal and the generator table, and
  recommends version 7 without requiring it.
- The new rows of `docs/reference/testing.md` name tests that exist, and the page's
  counts match the suites. `gleam-double/SPEC-NOTES.md`'s item 9 and G68 say what landed.
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

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). The
block runs under `sh`, which splits `$trees` into its paths; a shell that does not split
it, such as zsh, makes `rg` exit 2. A search that must find nothing passes only on `rg`'s
status 1, never on an error, and follows a positive control over the same files. No
command feeds a pipe. Drop from `trees` any path the base does not hold, and say so; a
missing path makes `rg` exit 2, which is a failure, not a pass. The tickets,
`CHANGELOG.md` and the two `SPEC-NOTES.md` files are not searched: they record what was
asked and decided. At step 2 the id search must match: `rg` exits 0, the test after it
prints "exit 1", and the output is the inventory. At the close it must find nothing: the
test prints "exit 0".

```sh
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
# the rule is stated in the module: at step 2 this exits 1, at the close 0
rg -n -i 'consumer id is a UUID' docs/specs/pachinko.allium; echo "exit $?"
trees='crates loadgen/src fuzz README.md docs gleam-double/src gleam-double/test gleam-double/README.md double/pachinko_double double/tests double/README.md'
# the ids searched; if step 1 keeps a publisher id, use the commented line instead
ids='ConsumerId'; fields='consumer_id'; paths='consumers'; binds='consumers?'; names='consumer|nacked_by'
# ids='ConsumerId|PublisherId'; fields='consumer_id|publisher_id'; paths='consumers|publishers'; binds='consumers?|publishers?'; names='consumer|nacked_by|publisher'
h='[0-9a-fA-F]'
# a valid UUID: 8-4-4-4-12 hex digits in any case, octet 8 of the 10 variant, any version
uuid="$h{8}-$h{4}-$h{4}-[89abAB]$h{3}-$h{12}"
# a string literal that is not exactly a valid UUID
bad="\"(?!$uuid\")[^\"]*\""
# lines that carry the marker are skipped
keep='^(?!.*id-format: refused)'
s1="($ids)(::new|::parse)?\\(&?$bad"
s2="\"($fields)\" *: *$bad"
s3="/($paths)/(?![{\$]|$uuid([/ \"\`?]|\$))[^/ \"\`)'{]+"
s4="\\.(pull|try_pull|lease|lease_view|ack|nack|renew|release)\\(&?$bad"
s5="(?i:$binds)[a-z_0-9]*\\b[^=]*= *[\\[(&]*(\"[^\"]*\", *)*$bad"
# the Python double's printed lease and dead letter, not a keyword argument
s6="^(lease|dead letter): .*\\b($names)=(?!$uuid\\b)[^,\\s)]+"
# today's other helpers: the id first, or after one plain argument; a free pull takes no JSON body
s7="\\b(consumer|marks|wait_for_lapse|until_no_lease)\\(([a-z_&]+, *)?$bad|(?<![.\\w])pull\\(\"(?![{])(?!$uuid\")[^\"]*\""
idre="$keep.*($s1|$s2|$s3|$s4|$s5|$s6|$s7)"
# hand-made control: prints lines 1 to 18 and none of 19 to 23
ctl=$(mktemp)
printf '%s\n' \
  'ConsumerId::new("c1")' \
  'ConsumerId("")' \
  '{"consumer_id": "worker-1", "record_key": "k"}' \
  'POST /consumers/c1/pull' \
  'fixture.pull("c1", 10)' \
  'client.ack("nobody", &lease, "k")' \
  'const CONSUMERS: [&str; 3] = ["c0", "c1", "c2"];' \
  'lease: id=lease-1, pocket=pk, consumer=c1, status=active' \
  'dead letter: pocket=pk, key=k, nacked_by=c1, nacked_at=20' \
  'let unknown = LeaseActionError::UnknownConsumer(consumer("nobody"));' \
  'assert_eq!(marks(client, "c1"), [marked("k", "nacked")]);' \
  'let (gone, answer) = pull("c1");' \
  'ConsumerId("0192D6C8-0000-7000-8000-0000000000010")' \
  'ConsumerId("{0192d6c8-0000-7000-8000-000000000001}")' \
  'ConsumerId("00000000-0000-0000-0000-000000000000")' \
  'ConsumerId("ffffffff-ffff-ffff-ffff-ffffffffffff")' \
  'ConsumerId("0192d6c8000070008000000000000001")' \
  'ConsumerId("urn:uuid:0192d6c8-0000-7000-8000-000000000001")' \
  'ConsumerId::new("c") // id-format: refused' \
  'assert_eq!(refused(pull("{}"), 400), "bad_request"); Lease(id=lease_id, consumer=consumer, status=status)' \
  'ConsumerId("0192d6c8-0000-7000-8000-000000000001"), ConsumerId::new("4B1C5F0E-2A7D-4C3E-BF10-6D2E8A7B9C01")' \
  'ConsumerId("00000000-0000-0000-8000-000000000000"), ConsumerId("00000000-0000-f000-b000-000000000000")' \
  '{"consumer_id": "0192d6c8-0000-7000-8000-000000000001"}, /consumers/{consumer_id}/lease, /consumers/0192d6c8-0000-7000-8000-000000000001/pull' >| "$ctl"
rg -n --pcre2 "$idre" "$ctl"; echo "exit $?"
rm -f "$ctl"
# positive controls: each form is found in the trees (each exit 0)
rg -q "($ids)(::new)?\\(\"" $trees; echo "exit $?"
rg -q "\"($fields)\" *: *\"" $trees; echo "exit $?"
rg -q "/($paths)/" $trees; echo "exit $?"
rg -q '\.(pull|lease|ack|nack|renew|release)\(' $trees; echo "exit $?"
rg -q --pcre2 "(?i:$binds)[a-z_0-9]*\\b[^=]*= *[\\[(&]*\"" $trees; echo "exit $?"
rg -q --pcre2 "^(lease|dead letter): .*\\b($names)=" $trees; echo "exit $?"
rg -q --pcre2 "\\b(consumer|marks|wait_for_lapse|until_no_lease|pull)\\(([a-z_&]+, *)?\"" $trees; echo "exit $?"
# no consumer id that is not a valid UUID is left: at the close rg must exit 1, so this prints "exit 0"
rg -n --pcre2 "$idre" $trees
[ "$?" -eq 1 ]; echo "exit $?"
# the refusal tests exist and are marked (exit 0); list each line in the hand-back
rg -n 'id-format: refused' $trees; echo "exit $?"
just plan-spec pachinko; echo "exit $?"
just test; echo "exit $?"
just coverage; echo "exit $?"
just effects; echo "exit $?"
just semver; echo "exit $?"
just test-loadgen; echo "exit $?"
# only if step 1 took a crate
just deny; echo "exit $?"
just deps-unused; echo "exit $?"
just check-docs; echo "exit $?"
# only if the base holds the double: prints "Ran <n> tests", ends "OK"
just python-double; echo "exit $?"
# only if the base holds the double: ends "<n> tests run, <n> passed", "coverage faults: 0"
just gleam-double; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

When this ticket was written, at commit `e92abc1` and over the trees its writer could
read (`crates` without `crates/libpachinko/src`, and without `double/pachinko_double`
and `double/tests`), the block above ran under `sh`: the module search exited 1, the
control printed lines 1 to 18 and exited 0, each positive control exited 0, the id
search exited 0 with the 1,062 lines of "The ids the change must rewrite" and its test
printed "exit 1", and the marker search exited 1.

## Hand-back notes

### Baseline

### T43's decision, T45's answers and the maintainer's answers (step 1)

### The sources, re-read (step 1)

### The merged state and the inventory (step 2)

### The specification

### Red tests

### Verification output

### Adversarial reviews

### Handed back

## Open points

- The nine points of "What the ruling leaves open" are step 1's to settle, after T43's
  decision and T45's answers.
- The checker's verdict on each form was not known when the ticket was written; step 1
  produces it.
- Every source fact except Python 3.14's `uuid.uuid7()` and the Go 1.27 `uuid` package
  is step 1's or step 9's to re-read: the RFC 9562 quotes, the prior art, the parsers'
  leniencies, the binary formats and the generator rows. Whether the Rust `uuid` crate
  has a hyphenated-only parser, and its minimum Rust version at 1.29.x, were not checked.
- `crates/libpachinko/src/`, `double/pachinko_double/` and `double/tests/` were not read
  when the ticket was written: `ConsumerId`'s definition and callers there, the Python
  double's consumer type and the count of non-UUID ids there are step 2's.
- Whether a test tagged ahead of its catalog row fails either double's coverage check is
  step 2's to find out.
- Whether the patterns, `s6` and `s7` above all, match anything but consumer ids in
  `double/pachinko_double/` and `double/tests/`, which the writer did not read, is step
  2's to find out; a pattern that does is narrowed, never marked.
- Whether the doubles move in this ticket depends on the base step 0 finds.
- RFC 9562 §8's point, whether holding a lease id ever grants authority, is T43's
  amendment's open point, not this ticket's.
