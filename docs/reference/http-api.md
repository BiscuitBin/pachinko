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
| `PUT /pockets/{pocket_key}/records/{record_key}` | `{"publisher_id": "p1", "rank": 1, "payload": "aGVsbG8="}` | `Publish` | 200 `{"outcome": "created"}`, or `"new_key"`, `"superseded"`, `"stale"`; 404 `unknown_publisher`; 409 `equal_rank`; 400 `bad_request` |
| `POST /consumers/{consumer_id}/pull` | `{"limit": 10}` | `Pull` | 200 with a batch, or 200 `{"outcome": "no_pocket"}`; 404 `unknown_consumer`; 400 `bad_request` |
| `GET /consumers/{consumer_id}/lease` | none | the `Consuming` surface's `exposes` | 200 with the lease, or 204 with no body when the consumer holds none; 404 `unknown_consumer` |
| `POST /leases/{lease_id}/ack` | `{"consumer_id": "c1", "record_key": "k1"}` | `Ack` | 200 `{"outcome": "settled"}` or `{"outcome": "deferred"}`; 404 `unknown_consumer`; 409 `lease_not_active`; 403 `lease_not_yours`; 404 `record_not_held`; 400 `bad_request` |
| `POST /leases/{lease_id}/nack` | `{"consumer_id": "c1", "record_key": "k1"}` | `Nack` | as ack |
| `POST /leases/{lease_id}/renew` | `{"consumer_id": "c1"}` | `Renew` | 200 `{"expires_at": 1762500300000}`; 404 `unknown_consumer`; 409 `lease_not_active`; 403 `lease_not_yours`; 400 `bad_request` |
| `POST /leases/{lease_id}/release` | `{"consumer_id": "c1"}` | `Release` | 200 `{"pocket": "queued"}` or `{"pocket": "idle"}`; 404 `unknown_consumer`; 409 `lease_not_active`; 403 `lease_not_yours`; 400 `bad_request` |
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
`--lease-ttl`; a refused action moves nothing. A release ends the lease, returns its
held and deferred records to queued in place, and requeues the pocket at the back
(`queued`) or, with no records left, leaves it idle until a publish revives it (`idle`).
No action checks `expires_at` against the clock: a late action the owner applies before
the lapse succeeds (decision 0019).

A lease route checks, in this order, the order of the rules' `requires`: the path's
`lease_id` parses as 32 lower-case hexadecimal digits (400 `bad_request`, before
anything reaches the owner), the consumer has joined (404 `unknown_consumer`), the lease
is active (409 `lease_not_active`, for a lease that has ended and one that never existed
alike), the lease is the consumer's (403 `lease_not_yours`), and, for ack and nack, the
lease's pocket holds the record (404 `record_not_held`).

Every command reaches the queue through one owner task, in arrival order. A pull whose
client has disconnected before the owner reaches it is skipped, so no lease is granted
to nobody; a client that disconnects while its pull is being applied leaves a lease that
nobody acts on: its lease id never reached the client, and the lease read does not
carry one, so no route ends it until lapses arrive with T26.

A request body is at most 2 MiB, axum's default: a longer body is refused with 400
`bad_request`, so a payload over about 1.5 MiB before base64 encoding is refused. T27
replaces that bound with the service's own request-shape limits.

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
| `bad_request` | 400 | The body is not the JSON the route takes, a field is missing, the body is over 2 MiB, the payload is not base64, or a lease id is not 32 lower-case hexadecimal digits. |
| `not_ready` | 503 | The queue's owner has stopped, or the library refused with a variant the map does not know, which is also logged as an error. T29 adds readiness. |

## Not exposed

The superseded records and the dead letters nacks create are kept; they decay only once T26's sweep
arrives, so until then they stay in memory for the life of the process. No route reads
them: who reads them is the specification's second open question. A publish is never refused
for want of room, and a pull is never refused for want of a pocket.

## Related pages

- [Decision 0019: The service runtime: tokio, one owner task and HTTP/JSON](../decisions/0019-service-runtime-tokio-and-http.md)
- [Configuration](configuration.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Architecture](../explanation/architecture.md)
- [Security model](../explanation/security-model.md)
