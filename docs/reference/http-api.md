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
| `POST /leases/{lease_id}/ack` | `{"consumer_id": "c1", "record_key": "k1"}` | `Ack` | 200 `{"outcome": "settled"}` or `{"outcome": "deferred"}`; 404 `unknown_consumer`; 409 `lease_not_active`; 403 `lease_not_yours`; 404 `record_not_held`; 400 `bad_request` |
| `POST /leases/{lease_id}/nack` | `{"consumer_id": "c1", "record_key": "k1"}` | `Nack` | as ack |
| `POST /leases/{lease_id}/renew` | `{"consumer_id": "c1"}` | `Renew` | 200 `{"expires_at": 1762500300000}`; 404 `unknown_consumer`; 409 `lease_not_active`; 403 `lease_not_yours`; 400 `bad_request` |
| `POST /leases/{lease_id}/release` | `{"consumer_id": "c1"}` | `Release` | 200 `{"pocket": "queued"}` or `{"pocket": "idle"}`; 404 `unknown_consumer`; 409 `lease_not_active`; 403 `lease_not_yours`; 400 `bad_request` |
| `GET /healthz` | none | none | 200 `ok` as text |
| `GET /readyz` | none | none | 204 with no body while serving; 503 `not_ready` otherwise |

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
since the Unix epoch. `lease_id` is 32 lower-case hexadecimal digits, and the four lease
routes name it in their path. The lease read carries no `lease_id`, because the
specification's `exposes` does not; a consumer keeps the one its batch carried.

The lease id fences each action to the lease it was pulled under. An ack, nack, renew or
release succeeds only while that lease is active and the consumer's: once a lease has
ended it leaves the service's registry, so an action naming it is `lease_not_active`
even when the same consumer has claimed the same pocket again under a new lease. An ack
removes a held record whose held version is still the latest (`settled`), or, when a
newer version arrived while it was held, defers that newer version to the back of the
pocket until the lease ends (`deferred`). A nack does the same and keeps the held
version as a dead letter. Ack, nack and renew each move `expires_at` to now plus
`--lease-ttl-ms`; a refused action moves nothing. A release ends the lease, returns its
held and deferred records to queued in place, and requeues the pocket at the back
(`queued`) or, with no records left, leaves it idle until a publish revives it (`idle`).
No action checks `expires_at` against the clock: a late action the owner applies before
the lapse succeeds (decision 0019).

A lease nobody pulls under, acks, nacks or renews lapses: the service sweeps every
`--sweep-ms`, and the sweep ends every lease whose `expires_at` has passed, returns its
held and deferred records to queued in place and keeps the pocket at the front of the
queue, so the next pull claims it. After `--max-consecutive-expiries` lapses in a row
the pocket parks instead and no pull claims it until a publish into it; a pocket whose
records were all acked or nacked goes idle below that count. The sweep is not instant:
a lease can stay active past `expires_at` for up to one sweep, plus the application of
the sweep in progress and of the command the owner is applying, and a sweep reads the
time once, so a lease expiring while one runs waits for the next. A pull, ack, nack or
renew the owner applies in that window, before the lapse, succeeds and extends the lease,
and a release applied in it succeeds by ending the lease and freeing its records; neither
is refused for being late (decision 0019). Arrival does not decide: a sweep the owner
takes up first ends the lease, and an action already waiting then gets
`lease_not_active`. Recovery after a consumer dies therefore takes `--lease-ttl-ms` plus
that window. Once the lapse is applied, the old lease is `lease_not_active` to every
route, and `GET /consumers/{consumer_id}/lease` answers 204.

A lease route checks, in this order, the order of the rules' `requires`: the path's
`lease_id` parses as 32 lower-case hexadecimal digits (400 `bad_request`, before
anything reaches the owner), the consumer has joined (404 `unknown_consumer`), the lease
is active (409 `lease_not_active`, for a lease that has ended and one that never existed
alike), the lease is the consumer's (403 `lease_not_yours`), and, for ack and nack, the
lease's pocket holds the record (404 `record_not_held`).

Every command reaches the queue through one owner task, in arrival order. A pull whose
client has disconnected before the owner reaches it is skipped, so no lease is granted
to nobody. A client that disconnects while its pull is being applied, or whose pull is
answered 408, never sees the lease id, and the lease read does not carry one. The
records of that lost batch stay held, visible through the lease read, and are not handed
out again: the consumer's next pull is a pull within the same lease, which returns only
records newly held, extends the lease and carries its id. To recover, pull with
`{"limit": 0}`, which holds nothing more but returns the lease id, then release the
lease, or act on the held records under it; otherwise the lease lapses at its
`expires_at` once the consumer stops pulling.

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

## `GET /readyz`

`GET /healthz` is liveness: 200 whenever the process serves HTTP. `GET /readyz` is
readiness, the route a load balancer or an orchestrator's probe should read. It reports
the queue owner's state:

| State | Reply |
| --- | --- |
| `starting` | 503 `{"error": "not_ready", "message": "starting"}`: the listener is bound but the owner has not begun taking commands. |
| `serving` | 204 with no body. |
| `stopping` | 503 `{"error": "not_ready", "message": "stopping"}`: shutdown has begun, or a task of the server has ended. |

Neither probe is bounded by the request timeout, and neither is logged above `debug`.

## Request timeout

Every route but `/healthz` and `/readyz` is bounded by `--request-timeout-ms` (30000 by
default). A handler still running at the bound is dropped and answered 408
`request_timeout`. A command the handler already sent to the owner is not withdrawn: the
owner still applies it. A client whose publish timed out may therefore find the record
applied. Retrying is safe. A `stale` outcome, or on the single `PUT` a 409 `equal_rank`
at the same rank, means a record at that rank or higher is already stored, whether the
first attempt or another publish stored it. It does not prove the first attempt landed:
a 408 can come before the command reaches the owner, for example while the body is still
being read. The usual cause is an owner far behind on its commands.

The same flag bounds idle connections. A connection that sends nothing for one timeout
is closed. One that has sent bytes and is still owed a reply is closed two timeouts
after its last byte if no reply has been written. A request whose head arrived in full
always gets its 408 first. A head that never completes gets no reply; the connection is
just closed.

## Shutdown

On SIGTERM or SIGINT, `GET /readyz` answers 503 `stopping` at once, the listener stops
accepting, and requests in flight finish. Connections still open after
`--shutdown-grace-ms` (5000 by default) are cut without a reply, and the owner stops
taking commands. Anything still running one more grace period later is aborted, and
`serve` exits non-zero. A grace of 0 cuts at once, waits for the server's tasks to end on
the cut and aborts none, so a clean stop still exits 0. A client that sees a 503, a refused connection or a cut
connection during the window retries once the service is back. The queue is in memory,
so a restarted service starts empty.

## Errors

Every refusal has the body `{"error": "<code>", "message": "<text>"}`, and one
exhaustive map in `crates/pachinko/src/http.rs` gives each code its status:

| Code | Status | When |
| --- | --- | --- |
| `publisher_exists` | 409 | The publisher id has already joined. |
| `consumer_exists` | 409 | The consumer id has already joined. |
| `unknown_publisher` | 404 | A publish names a publisher that has not joined. |
| `unknown_consumer` | 404 | A pull, lease read or lease action names a consumer that has not joined. |
| `equal_rank` | 409 | The rank equals the stored record's. No rule covers it; the message says the refusal is a placeholder for the specification's first open question. |
| `lease_not_active` | 409 | A lease action names a lease that has ended or never existed. |
| `lease_not_yours` | 403 | A lease action names another consumer's active lease. |
| `record_not_held` | 404 | An ack or nack names a record the lease's pocket does not hold. |
| `bad_request` | 400 | The body is not the JSON the route takes, a field is missing, a batch's `records` is empty, a payload is not base64, or a lease id is not 32 lower-case hexadecimal digits. |
| `too_many_rows` | 413 | A batch carries more rows than `--max-batch-rows`. |
| `payload_too_large` | 413 | A payload decodes to more bytes than `--max-payload-bytes`; the message names the row. |
| `body_too_large` | 413 | The request body is longer than `--max-body-bytes`. |
| `not_ready` | 503 | `GET /readyz` while the service is starting or stopping; any other route once the queue's owner has stopped; or the library refused with a variant the map does not know, which is also logged as an error. No route answers 500. |
| `request_timeout` | 408 | The handler ran past `--request-timeout-ms`. A command it sent may still be applied; see [Request timeout](#request-timeout). |

## Not exposed

The superseded records and the dead letters nacks create are kept until they decay, at
`--superseded-ttl-ms` and `--dead-letter-ttl-ms` after they were shed or nacked, on the
first sweep after that. No route reads them: who reads them is the specification's second open question. A publish is never refused
for want of room (the specification's `NeverRefusedForCapacity`): a service under load
makes the request wait for the owner, and the 413s above refuse a request's shape only.
A pull is never refused for want of a pocket.

## Related pages

- [Decision 0019: The service runtime: tokio, one owner task and HTTP/JSON](../decisions/0019-service-runtime-tokio-and-http.md)
- [Configuration](configuration.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Architecture](../explanation/architecture.md)
- [Security model](../explanation/security-model.md)
