# pachinko

[![CI](https://github.com/BiscuitBin/pachinko/actions/workflows/ci.yml/badge.svg)](https://github.com/BiscuitBin/pachinko/actions/workflows/ci.yml)

A small work queue, written in Rust and served over HTTP/JSON, that hands each consumer
an exclusive lease on one *pocket* of records at a time, and lets a higher-ranked version
of a record replace the one still waiting.

**Status: early, pre-1.0.** Publishers publish keyed, ranked records into pockets; a
higher rank supersedes the waiting record in place. A consumer pulls under a lease on one
pocket, then acks, nacks, renews or releases. A lease left alone lapses, its pocket keeps
its place in the queue, and repeated lapses park the pocket until a publish into it is accepted.
Superseded records and dead letters are kept for 12 hours by default, then decay. It does
not persist anything yet: the queue lives in memory and a restart loses it. It has no
authentication or TLS and listens on `127.0.0.1:8080` by default. One deployment is one
queue. Superseded records and dead letters cannot be read back; who reads them is an open
question. Neither crate is published to crates.io.

## How it works

Publishers put keyed, ranked records into pockets, and each consumer works through one
pocket at a time under an exclusive lease.

```mermaid
flowchart LR
    P["Publishers"] -- "(1) PUT record at rank N<br/>a higher rank replaces it" --> A
    subgraph S ["pachinko serve"]
        A["pocket tenant-a"]
        T(["lease timer"])
        T -. "(4) or 5 min with no pull, ack, nack or renew (by default):<br/>the lease lapses, the pocket keeps its place" .-> A
    end
    A <== "(2) pull · (3) ack or nack · (4) release" ==> C["Consumer"]
    classDef leased fill:#fde68a,stroke:#b45309,color:#000
    class A leased
```

The everyday loop is the one any work queue has: publish, pull, ack.

```mermaid
sequenceDiagram
    participant P as Publisher
    participant Q as pachinko
    participant C1 as Consumer c1
    P->>Q: join as p1
    C1->>Q: join as c1
    P->>Q: PUT tenant-a/k1 at rank 1
    Q-->>P: created
    C1->>Q: pull
    Q-->>C1: batch [k1 rank 1] and a lease on tenant-a
    C1->>Q: ack k1
    Q-->>C1: settled
```

One pocket has one consumer at a time. A second consumer gets a different pocket, and a
consumer that goes quiet loses its lease without losing the pocket's turn.

```mermaid
sequenceDiagram
    participant Q as pachinko
    participant C1 as Consumer c1
    participant C2 as Consumer c2
    Note over Q: tenant-a and tenant-b each hold a record
    C1->>Q: pull
    Q-->>C1: lease on tenant-a
    C2->>Q: pull
    Q-->>C2: lease on tenant-b, never tenant-a
    C2->>Q: ack k1
    Q-->>C2: settled
    Note over C1: goes quiet
    Note over Q: the lease timer lapses c1's lease
    Note over Q: tenant-a keeps its place at the front
    C2->>Q: release
    Q-->>C2: idle, tenant-b has no records left
    C2->>Q: pull
    Q-->>C2: lease on tenant-a
```

The newest version of a record wins. A higher rank replaces the waiting version, a lower
one is answered `stale` and leaves the waiting version in place, and a version that
arrives while its record is held waits for the lease.

```mermaid
sequenceDiagram
    participant P as Publisher
    participant Q as pachinko
    participant C1 as Consumer c1
    P->>Q: PUT tenant-a/k1 at rank 1
    Q-->>P: created
    P->>Q: PUT tenant-a/k1 at rank 2
    Q-->>P: superseded, rank 2 replaces rank 1
    P->>Q: PUT tenant-a/k1 at rank 1, late
    Q-->>P: stale, rank 2 stays
    C1->>Q: pull
    Q-->>C1: lease on tenant-a, batch [k1 rank 2]
    P->>Q: PUT tenant-a/k1 at rank 3
    Q-->>P: superseded
    Note over Q: c1 still holds rank 2
    Note over Q: rank 3 waits for the lease
    C1->>Q: ack k1
    Q-->>C1: deferred
    C1->>Q: release
    Q-->>C1: queued
    C1->>Q: pull
    Q-->>C1: lease on tenant-a, batch [k1 rank 3]
```

A pocket whose lease ends with no records left goes idle until a publish into it, and a
pocket whose lease lapses three times in a row (by default) parks until a publish into it
is accepted. Expiry runs on a sweep, so a late action the service applies before the
lapse is not refused for being late; its other checks still apply
([decision 0019](docs/decisions/0019-service-runtime-tokio-and-http.md)). The
[HTTP API](docs/reference/http-api.md) has every route and reply, and the specification,
[`docs/specs/pachinko.allium`](docs/specs/pachinko.allium), decides the behaviour.

## Run it

```console
$ cargo run -p pachinko --locked -- serve
listening on 127.0.0.1:8080
$ curl -s -o /dev/null -w '%{http_code}\n' -X POST http://127.0.0.1:8080/publishers/p1
201
$ curl -s -o /dev/null -w '%{http_code}\n' -X POST http://127.0.0.1:8080/consumers/c1
201
$ curl -s -X PUT http://127.0.0.1:8080/pockets/tenant-a/records/k1 \
    -H 'content-type: application/json' \
    -d '{"publisher_id": "p1", "rank": 1, "payload": "aGVsbG8="}'
{"outcome":"created"}
$ curl -s -X POST http://127.0.0.1:8080/consumers/c1/pull \
    -H 'content-type: application/json' -d '{"limit": 10}'
{"outcome":"batch","lease_id":"d8fb1ffb1bd779af0000000000000001","pocket_key":"tenant-a","expires_at":1794468160610,"records":[{"record_key":"k1","rank":1,"payload":"aGVsbG8="}]}
```

Run the curl calls from a second terminal; if port 8080 is taken, `--listen 127.0.0.1:0`
picks a free port and prints it. [Test and debug](docs/how-to/test-and-debug.md#run-the-service)
adds the lease read and the health check, and
[Build the container image](docs/how-to/build-the-container-image.md) packages the service.

## Using pachinko

- [HTTP API](docs/reference/http-api.md): every route, reply and error code, acks included.
- [Configuration](docs/reference/configuration.md): the flags `serve` takes and their defaults.
- [Security model](docs/explanation/security-model.md): what an unauthenticated service exposes.
- [The Rust library](crates/libpachinko/README.md): `libpachinko`, the queue without the
  server, with its [API reference](https://biscuitbin.github.io/pachinko/).
- [The specification](docs/specs/pachinko.allium): the behaviour, and its open questions.
- [Decision 0019](docs/decisions/0019-service-runtime-tokio-and-http.md): why tokio, one owner task and HTTP/JSON.
- [The handbook](docs/README.md): development, architecture and operations.

## Contributing

Follow [Develop locally](docs/how-to/develop-locally.md) for the prerequisites, then run
`just initialize`, which needs the network, and `just check`, which is read-only. Read
[AGENTS.md](AGENTS.md) before changing anything; work is planned as tickets under
`tickets/`, each on its own branch and worktree. [CHANGELOG.md](CHANGELOG.md) records
changes, and [SECURITY.md](SECURITY.md) says how to report a vulnerability privately.

## Licence

Both crates are licensed under Apache-2.0. See [LICENSE](LICENSE).
