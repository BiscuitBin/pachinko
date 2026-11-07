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
| `unknown_consumer` | 404 | A pull or lease read names a consumer that has not joined. |
| `equal_rank` | 409 | The rank equals the stored record's. No rule covers it; the message says the refusal is a placeholder for the specification's first open question. |
| `bad_request` | 400 | The body is not the JSON the route takes, a field is missing, the body is over 2 MiB, or the payload is not base64. |
| `not_ready` | 503 | The queue's owner has stopped, or the library refused with a variant the map does not know, which is also logged as an error. T29 adds readiness. |

## Not exposed

The superseded records and dead letters are kept; they decay only once T26's sweep
arrives, so until then they stay in memory for the life of the process. No route reads
them: who reads them is the specification's second open question. A publish is never refused
for want of room, and a pull is never refused for want of a pocket.

## Related pages

- [Decision 0019: The service runtime: tokio, one owner task and HTTP/JSON](../decisions/0019-service-runtime-tokio-and-http.md)
- [Configuration](configuration.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Architecture](../explanation/architecture.md)
- [Security model](../explanation/security-model.md)
