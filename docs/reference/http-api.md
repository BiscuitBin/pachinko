---
title: "HTTP API"
kind: "reference"
audience: [user, contributor, maintainer, operator, agent]
canonical_for: [http_api]
requires: []
---

# HTTP API

`pachinko serve` answers the routes below, one per action of the `Publishing` and
`Consuming` surfaces of `docs/specs/pachinko.allium`. The behaviour behind each route is
the module's; the wire shape is a prototype's, chosen in
[decision 0019](../decisions/0019-tokio-and-axum-service.md), and this page owns it.
Bodies are JSON, payloads are base64, and keys travel in the path. There is no
authentication, TLS or rate limiting.

## Routes

| Method and path | Body | Surface action | Replies |
| --- | --- | --- | --- |
| `POST /publishers/{publisher_id}` | none | `PublisherJoins` | 201; 409 `publisher_exists` |
| `POST /consumers/{consumer_id}` | none | `ConsumerJoins` | 201; 409 `consumer_exists` |
| `PUT /pockets/{pocket_key}/records/{record_key}` | `{"publisher_id", "rank", "payload"}` | `Publish` | 200 `{"outcome": "created" or "new_key" or "superseded" or "stale"}`; 404 `unknown_publisher`; 409 `equal_rank` |
| `POST /consumers/{consumer_id}/pull` | `{"limit"}` | `Pull` | 200 with a batch, or 200 `{"no_pocket_available": true}`; 404 `unknown_consumer` |
| `POST /leases/{lease_id}/ack` | `{"consumer_id", "record_key"}` | `Ack` | 200 `{"outcome": "settled" or "deferred"}`; 409 `lease_not_active`; 403 `lease_not_yours`; 404 `record_not_held` |
| `POST /leases/{lease_id}/nack` | `{"consumer_id", "record_key"}` | `Nack` | as ack |
| `POST /leases/{lease_id}/renew` | `{"consumer_id"}` | `Renew` | 200 `{"expires_at"}`; 409 `lease_not_active`; 403 `lease_not_yours` |
| `POST /leases/{lease_id}/release` | `{"consumer_id"}` | `Release` | 200 `{"pocket": "queued" or "idle"}`; 409 `lease_not_active`; 403 `lease_not_yours` |
| `GET /consumers/{consumer_id}/lease` | none | the `Consuming` surface's `exposes` | 200 with the lease, or 204 when the consumer holds none; 404 `unknown_consumer` |
| `GET /healthz` | none | none | 200 `ok` |

A batch and a lease share one shape: `{"lease_id", "pocket_key", "expires_at",
"records": [{"record_key", "rank", "payload"}]}`. A batch's records are the ones it
handed out, oldest first; a lease's records are the ones it holds, at the rank and
payload they were handed out with. `lease_id` is 32 hexadecimal digits, and every ack,
nack, renew and release names it; an action naming a lease that has ended is refused
with 409 `lease_not_active`, even when the same consumer holds the same pocket again.
`expires_at` is milliseconds since the UNIX epoch. A `limit` that is not positive yields
an empty batch, not an error.

Leases lapse, and superseded records and dead letters decay, when the sweep runs, every
`--sweep-ms` ([Configuration](configuration.md)). A lease can therefore stay active for up
to one sweep past its `expires_at`, and an action that arrives in that window succeeds and
extends it: the module's eighth open question leaves that to processing order. A request
body is at most 2 MiB, axum's default, so a payload over about 1.5 MiB before base64
encoding is refused with 400 `bad_request`, and so is any longer body, a pull's included.

## Errors

Every refusal has the body `{"error": "<name>", "message": "<text>"}`. The name is the
library's `Error` variant in snake case, and one function in `crates/pachinko/src/http.rs`
maps each to its status:

| Name | Status | When |
| --- | --- | --- |
| `publisher_exists`, `consumer_exists` | 409 | The id has already joined. |
| `unknown_publisher`, `unknown_consumer` | 404 | The id has not joined. |
| `equal_rank` | 409 | The rank equals the stored record's. No rule covers it; the message says the rejection is a placeholder for the module's first open question. |
| `lease_not_active` | 409 | The lease has ended, or never existed. |
| `lease_not_yours` | 403 | The lease is another consumer's. |
| `record_not_held` | 404 | The lease does not hold that key. |
| `bad_request` | 400 | The body is not the JSON the route takes or is over 2 MiB, the lease id is not 32 hexadecimal digits, or the payload is not base64. |

The checks run in the order each rule's `requires` reads: the lease is active, then it
is the consumer's, then the record is held.

## Not exposed

The superseded records and dead letters are kept and decay, but no route reads them:
who reads them is the module's second open question. A pull is never refused for want of
a pocket, and a publish is never refused for want of room.

## Related pages

- [Decision 0019: The service runs on tokio and serves HTTP with axum](../decisions/0019-tokio-and-axum-service.md)
- [Configuration](configuration.md)
- [Test and debug](../how-to/test-and-debug.md)
- [Specifications](../explanation/specifications.md)
