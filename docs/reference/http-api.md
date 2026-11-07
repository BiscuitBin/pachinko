---
title: "HTTP API"
kind: "reference"
audience: [user, contributor, maintainer, operator, agent]
canonical_for: [http_api]
requires: []
---

# HTTP API

`pachinko serve` answers the routes below over HTTP/1.1, one per action of the
`Publishing` and `Consuming` surfaces of `docs/specs/pachinko.allium` that the service
exposes so far. The behaviour behind each route is the specification's; the wire shape
was chosen in [decision 0019](../decisions/0019-service-runtime-tokio-and-http.md), and
this page owns it. Bodies are JSON, payloads are base64 (the standard alphabet, with
padding), and keys travel in the path, percent-encoded. There is no authentication, TLS
or rate limiting; [Security model](../explanation/security-model.md) says what that
exposes. The flags `serve` takes are in [Configuration](configuration.md#service-flags).

## Routes

| Method and path | Body | Surface action | Replies |
| --- | --- | --- | --- |
| `POST /publishers/{publisher_id}` | none | `PublisherJoins` | 201 with no body; 409 `publisher_exists` |
| `POST /consumers/{consumer_id}` | none | `ConsumerJoins` | 201 with no body; 409 `consumer_exists` |
| `PUT /pockets/{pocket_key}/records/{record_key}` | `{"publisher_id": "p1", "rank": 1, "payload": "aGVsbG8="}` | `Publish` | 200 `{"outcome": "created"}`, or `"new_key"`, `"superseded"`, `"stale"`; 404 `unknown_publisher`; 409 `equal_rank`; 400 `bad_request`; 413 `payload_too_large`, `body_too_large` |
| `POST /publishers/{publisher_id}/publish` | `{"records": [{"pocket_key": "tenant-a", "record_key": "k1", "rank": 1, "payload": "aGVsbG8="}]}` | `Publish`, once per row | 200 `{"outcomes": [...]}`, one per row; 404 `unknown_publisher`; 400 `bad_request`; 413 `too_many_rows`, `payload_too_large`, `body_too_large` ([Batch publish](#batch-publish)) |
| `POST /consumers/{consumer_id}/pull` | `{"limit": 10}` | `Pull` | 200 with a batch, or 200 `{"outcome": "no_pocket"}`; 404 `unknown_consumer`; 400 `bad_request` |
| `GET /consumers/{consumer_id}/lease` | none | the `Consuming` surface's `exposes` | 200 with the lease, or 204 with no body when the consumer holds none; 404 `unknown_consumer` |
| `GET /healthz` | none | none | 200 `ok` as text |

`rank` is a JSON integer read as a signed 64-bit number, so every value from
`-9223372036854775808` to `9223372036854775807` round-trips exactly. A `limit` of zero or
less claims a pocket and returns an empty batch; a `limit` above `--max-pull-batch`
returns at most that many. Finding no pocket is data, not an error.

A batch is:

```json
{"outcome": "batch", "lease_id": "8c1f0e5a9b7d42e10000000000000001", "pocket_key": "tenant-a",
 "expires_at": 1762500000000, "records": [{"record_key": "k1", "rank": 1, "payload": "aGVsbG8="}]}
```

Its records are the ones this pull handed out, oldest first. A lease is
`{"pocket_key", "expires_at", "records"}` in the same shapes, its records the ones it
holds, at the rank and payload they were handed out with. `expires_at` is milliseconds
since the Unix epoch. `lease_id` is 32 hexadecimal digits; the ack, nack, renew and
release actions, which arrive with T25, name it. The lease read carries no `lease_id`,
because the specification's `exposes` does not; a consumer keeps the one its batch
carried.

Every command reaches the queue through one owner task, in arrival order. A pull whose
client has disconnected before the owner reaches it is skipped, so no lease is granted
to nobody; a client that disconnects while its pull is being applied leaves a lease that
nobody acts on, which no route ends until lapses arrive with T26.

A request body any route reads is at most `--max-body-bytes`, 16 MiB unless set: a longer
body is refused with 413 `body_too_large`, by its `Content-Length` before it is read, or,
without one, once the read passes the limit, whatever its content type. A payload, on
the single `PUT` or on a batch row, is at most `--max-payload-bytes` once decoded, 1 MiB
unless set; a longer one is 413 `payload_too_large`, and the single `PUT` counts as row 1.
These are refusals of a request's shape, never of the queue's room.

## Batch publish

`POST /publishers/{publisher_id}/publish` publishes many records in one request:

```json
{"records": [{"pocket_key": "tenant-a", "record_key": "k1", "rank": 1, "payload": "aGVsbG8="},
             {"pocket_key": "tenant-a", "record_key": "k1", "rank": 1, "payload": "aGVsbG8="}]}
```

It answers 200 with one outcome per row, in row order, each `created`, `new_key`,
`superseded`, `stale` or `equal_rank`:

```json
{"outcomes": ["created", "equal_rank"]}
```

An equal rank is answered for its row alone, where the single `PUT` answers 409
`equal_rank`; it is still the placeholder for the specification's first open question.
Rows apply in order and independently, as one command to the owner, so no other request
runs between two rows: a row's outcome never undoes an earlier row, and a batch is never
undone as a set.

The whole request is refused, before any row applies, by the first of these checks that
fails:

| Check | Refusal |
| --- | --- |
| The body exceeds `--max-body-bytes`, by its `Content-Length` or as read | 413 `body_too_large` |
| The body is not the JSON the route takes, or `records` is empty | 400 `bad_request` |
| More rows than `--max-batch-rows` | 413 `too_many_rows` |
| A row's payload is not base64, the first such row | 400 `bad_request`, naming the row |
| A row's decoded payload exceeds `--max-payload-bytes`, the first such row | 413 `payload_too_large`, naming the row |
| The publisher has not joined | 404 `unknown_publisher` |

Rows are counted from 1. Every row is checked for base64 before any row's size is
checked. An empty `records` is refused so that the first row always exists to find an
unknown publisher by.

The three caps are not chosen to fit together, and the body cap is the outer bound. A
1,000-row batch, the `--max-batch-rows` default, fits under the 16 MiB `--max-body-bytes`
default only when its base64 payloads average under about 16 KiB; a full batch of 1 MiB
payloads would be about 1.3 GiB and is refused as `body_too_large`. The load check
guarantees only that one maximum payload fits in a one-row batch or a single `PUT`
([Configuration](configuration.md#service-flags)).

## Errors

Every refusal has the body `{"error": "<code>", "message": "<text>"}`, and one
exhaustive map in `crates/pachinko/src/http.rs` gives each code its status:

| Code | Status | When |
| --- | --- | --- |
| `publisher_exists` | 409 | The publisher id has already joined. |
| `consumer_exists` | 409 | The consumer id has already joined. |
| `unknown_publisher` | 404 | A publish names a publisher that has not joined. |
| `unknown_consumer` | 404 | A pull or lease read names a consumer that has not joined. |
| `equal_rank` | 409 | The rank equals the stored record's. No rule covers it; the message says the refusal is a placeholder for the specification's first open question. |
| `bad_request` | 400 | The body is not the JSON the route takes, a field is missing, a batch's `records` is empty, or a payload is not base64. |
| `too_many_rows` | 413 | A batch carries more rows than `--max-batch-rows`. |
| `payload_too_large` | 413 | A payload decodes to more bytes than `--max-payload-bytes`; the message names the row. |
| `body_too_large` | 413 | The request body is longer than `--max-body-bytes`. |
| `not_ready` | 503 | The queue's owner has stopped, or the library refused with a variant the map does not know, which is also logged as an error. T29 adds readiness. |

## Not exposed

The superseded records and dead letters are kept; they decay only once T26's sweep
arrives, so until then they stay in memory for the life of the process. No route reads
them: who reads them is the specification's second open question. A publish is never refused
for want of room (the specification's `NeverRefusedForCapacity`): a service under load
makes the request wait for the owner, and the 413s above refuse a request's shape only.
A pull is never refused for want of a pocket.

## Related pages

- [Decision 0019: The service runtime: tokio, one owner task and HTTP/JSON](../decisions/0019-service-runtime-tokio-and-http.md)
- [Configuration](configuration.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Architecture](../explanation/architecture.md)
- [Security model](../explanation/security-model.md)
