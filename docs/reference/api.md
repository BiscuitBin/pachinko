---
title: "API reference"
kind: "reference"
audience: [user, contributor, maintainer, agent]
canonical_for: [api_reference]
requires: []
---

# API reference

The API reference is the one rustdoc generates from the doc comments of both crates,
`libpachinko` and `pachinko`. This page does not copy it: a list of items written out
here would be wrong after the first change. It says how to build the reference, where it
lands, where it is hosted, what the public API promises, and what a doc example is for,
and it maps each crate's modules so a reader knows where to look. The one thing it lists
in full is the service's Arrow Flight API, a wire contract rustdoc cannot see.

## Building it

`just doc` runs `cargo doc` over the workspace with every feature, no dependencies and
`--locked`, with `RUSTDOCFLAGS="-D warnings --cfg docsrs"`. The result opens at
`target/doc/libpachinko/index.html` for the library and `target/doc/pachinko/index.html`
for the service. It is gate 12 of `just check`, so a broken intra-doc link, a link to a
private item or an undocumented public item fails the gate rather than producing a page
with a hole in it.

## What is documented

`missing_docs` is set to warn in the `[workspace.lints.rust]` table, and both `just doc`
and `just clippy` turn warnings into errors. Removing the doc comment from one public
item fails both recipes with a `missing documentation for` error naming its kind. So
every public item carries documentation, and each crate root carries the overview. The
rustdoc lints in `[workspace.lints.rustdoc]` add broken and private intra-doc links,
which fail outright, and a missing crate-level doc and unescaped backticks, which warn.

## Doc examples are tests

[The agent contract](agent-contract.md) asks for an example on every public item, and
`just test-doc` (gate 10) compiles and runs each example, so an example that stops
being true stops the gate. Examples are not counted
toward the coverage floor, because the coverage tooling cannot measure doctests on a
stable toolchain. The division of labour follows: an example shows the contract to a
reader and proves it, and a unit test earns the coverage. See [Testing](testing.md).

## Hosted

The reference for `main` is hosted at <https://biscuitbin.github.io/pachinko/>, and a
push to `main` republishes it. It documents `main`, not a release, so it may describe an
item no release has. `just site` builds what is hosted: an empty `target/doc`, then
`just doc` with the flags above, then a root `index.html` that links to both crates'
pages, because stable rustdoc writes none. `just site` is not a gate; `just doc` is.
[Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md) has the workflow and the
rollback, and
[decision 0013](../decisions/0013-api-reference-on-github-pages.md) records why the
reference is hosted there.

## docs.rs

`crates/libpachinko/Cargo.toml` and `crates/pachinko/Cargo.toml` each carry a
`[package.metadata.docs.rs]` table: every feature, `--cfg docsrs`, and one target,
`x86_64-unknown-linux-gnu`. This is prospective. Both crates are `publish = false`
([decision 0006](../decisions/0006-apache-2-0.md)), so nothing is on docs.rs, and the
release process ([decision 0016](../decisions/0016-release-process.md)) keeps it so: a
tag cuts a GitHub Release, not a registry publish. Only a later, separately authorised
decision to publish would put a crate on docs.rs; until then the hosted site is the only
API reference.

## What the API promises

Every public type is `Send + Sync + 'static`, `Clone` and `Debug`, so a caller may move
a value between threads, copy it or print it. Public structs and enums are
`#[non_exhaustive]`, so a field or variant added later breaks no caller. These are
invariants of [the agent contract](agent-contract.md). A refusal is a `thiserror` enum
whose message names what was refused. The library performs no effect itself: its effect
traits sit beside their fakes, the service supplies the real implementations, and
[Architecture](../explanation/architecture.md) explains the boundary.
[Configuration](configuration.md) lists each crate's dependencies.

## What each crate exposes

The reference is the authority on items; this is the map to it. `libpachinko`
implements the one specification module, [`pachinko.allium`](../specs/pachinko.allium),
in five public modules:

- `clock`: `Timestamp`, whole milliseconds since the Unix epoch; the `Clock` trait, the
  library's first effect trait; and `FakeClock`, its fake.
- `config`: `Config`, the module's five figures with the module's defaults, and
  `Config::validate`, which checks the module's config invariants.
- `error`: the enums a refused trigger returns: `ConfigError`, `JoinError`,
  `PublishError`, `ConsumerError` and `ActionError`.
- `records`: the `PocketRecords` trait each pocket's records sit behind, `MemoryRecords`,
  its reference implementation, and the value types the queue and a record store
  exchange.
- `queue`: `Queue<R, C>`, generic over a record store and a clock, with one method per
  trigger the two surfaces provide, `tick` for lease lapses and decay, and `lease_view`
  for the `Consuming` surface's `exposes`; `LeaseId`, the fencing token every ack, nack,
  renew and release carries; and the value and outcome types the methods take and return.

The library's `testing` feature adds `records::contract::check_contract`, the behaviour
every record store must show, and `Queue::snapshot`, the inspection seam the tests read
the queue through. No normal build turns it on, but `just doc` builds with every
feature, so the hosted reference shows both. `libpachinko::version` still returns the
crate's version.

`pachinko` exposes:

- `run`, the command line the binary's `main` calls and nothing else, with its error
  `CliError`. With no arguments it writes `pachinko <version>`; `serve` serves the queue
  until an interrupt. [Configuration](configuration.md) lists the options.
- `serve`, `ServeOptions` and `ServeError`: the queue behind the Flight API below, its
  owner task and ticker held in a `JoinSet` that `serve` joins before it returns.
  [Architecture](../explanation/architecture.md) explains the design and
  [decision 0019](../decisions/0019-service-runtime-tokio-and-flight.md) records it.
- `clock::SystemClock`, the real `Clock`, and `arrow::ArrowRecords`, the real
  `PocketRecords`, which holds each pocket's records as one Arrow `RecordBatch`
  ([decision 0020](../decisions/0020-records-as-arrow-batches.md)); `arrow::batch_schema`
  and `arrow::held_records` describe and read the batches a pull hands out.

## The Flight API

`pachinko serve` speaks Arrow Flight over gRPC. The module behind it is private, so
rustdoc does not show it, and this section is its reference. Every body, ticket and
reply is an Arrow IPC stream. A body or ticket holds one batch of one row whose columns
are read by name and type, so a missing column, a wrong type, a null, a second batch or
trailing bytes is `INVALID_ARGUMENT`. A time on the wire, `expires_at` or `ended_at`, is
milliseconds since the Unix epoch: a `UInt64` in a batch, a decimal string in schema
metadata. The service authenticates no one and encrypts nothing;
[Security model](../explanation/security-model.md) says what that means.

| Flight method | Trigger | Request | Reply |
| --- | --- | --- | --- |
| `do_put` | `Publish` | A descriptor of type command whose bytes are the publisher id in UTF-8, then batches with `pocket_key` Utf8, `record_key` Utf8, `rank` Int64 and `payload` Binary, none null | One `PutResult` per input batch, whose `app_metadata` is an IPC stream with one `outcome` Utf8 row per input row: `new`, `superseded`, `stale` or `rejected` |
| `do_get` | `Pull` | A ticket with `consumer_id` Utf8 and `limit` Int64 | The held records, oldest first, as `record_key` LargeUtf8, `rank` Int64 and `payload` LargeBinary, with schema metadata `outcome` = `batch`, `lease_id`, `pocket_key` and `expires_at`; or, when no pocket is available, a schema-only stream with `outcome` = `no_pocket`. The records may span several `FlightData` messages after the schema's. A message of more than one record is at most 2 MiB, its offsets and ranks counted, so it stays under a gRPC client's default 4 MiB message limit; a message of one record is as large as that record makes it |
| `do_action` | One of the seven action types below | An `Action` whose type names the action and whose body is its one row | One result whose body is the action's reply; `status` may answer with several |
| `list_actions` | None | Empty | The seven action types, each with a one-line description |

Every other Flight method answers `UNIMPLEMENTED`. A put reads every batch before it
publishes any, so a malformed batch publishes nothing. Each input batch is then
published row by row, in order: an unknown publisher is refused before any row lands,
and an equal rank refuses its row alone, reported as `rejected`. That refusal is the
module's placeholder for its first open question, not a decision. A pull hands out at
most `max_pull_batch` records and none when `limit` is not positive, and an empty batch
is not an error.

| Action type | Trigger | Body | Reply |
| --- | --- | --- | --- |
| `publisher_joins` | `PublisherJoins` | `publisher_id` Utf8 | `outcome` Utf8 = `joined` |
| `consumer_joins` | `ConsumerJoins` | `consumer_id` Utf8 | `outcome` Utf8 = `joined` |
| `ack` | `Ack` | `consumer_id` Utf8, `lease_id` UInt64, `record_key` Utf8 | `outcome` Utf8 = `settled` or `deferred` |
| `nack` | `Nack` | `consumer_id` Utf8, `lease_id` UInt64, `record_key` Utf8 | `outcome` Utf8 = `settled` or `deferred` |
| `renew` | `Renew` | `consumer_id` Utf8, `lease_id` UInt64 | `expires_at` UInt64 |
| `release` | `Release` | `consumer_id` Utf8, `lease_id` UInt64 | `lease_id` UInt64, `ended_at` UInt64 |
| `status` | The `Consuming` surface's `exposes` | `consumer_id` Utf8 | Schema metadata `lease` = `active` or `none`, and for an active lease its `lease_id`, `pocket_key` and `expires_at`; one row per held record, as `record_key`, `rank` and `payload`. The held records may span several results, each an IPC stream with the same schema and metadata. A result of more than one record is at most 2 MiB, its schema, metadata, offsets and ranks counted, so it stays under a gRPC client's default 4 MiB message limit; a result of one record is as large as that record and the metadata make it |

A refusal is a gRPC status, and one the queue made carries the library error's text as
its message:

| Status | When |
| --- | --- |
| `NOT_FOUND` | The publisher or consumer has not joined. |
| `FAILED_PRECONDITION` | A join of an id that has already joined, or an action the rules reject: a lease that is not active, a lease that is another consumer's, or a record not held under the lease. |
| `INVALID_ARGUMENT` | A malformed body, ticket, descriptor or input batch, or an unknown action type. |
| `UNAVAILABLE` | The queue's owner has stopped, or shutdown began before the request's handler had begun or while an upload was still open. |
| `INTERNAL` | The service could not write its own reply, a programming error. |
| `UNIMPLEMENTED` | A Flight method outside this API. |

No method reads superseded records or dead letters: who reads them is the module's
second open question.

## Related pages

- [Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md)
- [Architecture](../explanation/architecture.md)
- [Testing](testing.md)
- [Purpose and scope](../project/purpose-and-scope.md)
- [Decision 0013: The API reference on GitHub Pages](../decisions/0013-api-reference-on-github-pages.md)
- [Decision 0019: The service runs on tokio and speaks Arrow Flight](../decisions/0019-service-runtime-tokio-and-flight.md)
- [Decision 0020: Records as Arrow batches in the service](../decisions/0020-records-as-arrow-batches.md)
- [Security model](../explanation/security-model.md)
- [Configuration](configuration.md)
