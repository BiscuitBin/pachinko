---
title: "Decision 0020: A held record stays held until its lease ends"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_held_set]
requires: []
---

# Decision 0020: A held record stays held until its lease ends

## Context

Decisions about what pachinko does belong in the specification, and this one is there:
`docs/specs/pachinko.allium` states it in the Publish comment, `DeferBehindHeldVersion`,
the ack and nack rules, the "Lease end" rules with their reconciliation table, and the
`Consuming` guarantees `HeldSetStableUntilLeaseEnds`, `MarksStandHoweverLeaseEnds` and
`NothingLostWhileHeld`. The maintainer asked on 2026-11-16, at step 1 of ticket T35, for
a record of it as well, because it is a key decision: a terse one that says why and
leaves the rules to the specification.

Before it, an ack or a nack removed the held record at once. The wave's code review
found that a supersede of a held record shed the version the consumer still held, and
that a later nack dead-lettered that version again, so one version was both superseded
and a dead letter (open question 10). The module already asked what a key acked under a
lease and published again should be (open question 9): the rules made it a new record,
which the same lease could pull a second time.

## Decision

The maintainer's, in an elicitation on 2026-11-13, recorded in ticket T35. A record
handed out under a lease stays in the consumer's held set, at the rank and payload it
was handed out with, until the lease ends by release or by lapse. An ack or a nack only
marks it, and the first mark stands. While the lease is active no held version is shed,
superseded or dead-lettered: a newer version waits behind it, and a still newer one
replaces only the waiting version. The lease's end reconciles every held record by its
mark. The lease keeps its name; the properties are stated as guarantees rather than by
calling it a transaction.

Why:

- **The consumer works on a stable set.** What a lease was handed stays as it was handed
  until the lease ends, and the lease read shows it so: no publish changes or removes a
  version the consumer holds, and no key is handed out twice under one lease. The
  consumer gets repeatable reads of a snapshot that only its own pulls and marks change.
- **A mark is committed when it is processed, however the lease ends.** A lapse keeps
  every ack and nack, so work a consumer finished before it died is not redone, and a
  repeated ack or nack is safe to retry. Acked means consumed: an acked version is
  recorded nowhere.
- **Nothing held is shed or dead-lettered while the lease is active.** That removes the
  double shed: each version that leaves ends in exactly one place, the superseded queue
  for a version no consumer acked or nacked and the dead letters for a nacked one.
- **One rule judges a publish.** The kept record's rank decides whether the record is
  unsettled, acked or nacked, which closed question 9.
- **Total fairness of positions,** the maintainer's own note. A key no consumer acked or
  nacked keeps its place in the pocket's order, so its newer version takes the held
  version's position. Behind an acked or nacked version the newer version is a new
  arrival and is queued where it arrived.

## Consequences

- A pocket may hold two versions of a key: the held one and the highest that arrived
  since. The store keeps both, with the mark, the nack's time and the arrival position.
- Dead letters appear only when the lease ends, and an unsettled held version a newer
  one replaces is recorded as superseded then, not at the publish. A consumer that pulls
  in a loop and never releases keeps its acked records stored and on the lease read, and
  holds back its dead letters, until it releases or the lease lapses.
- The lease-end work moved from the ack to the release and the lapse: an ack or a nack
  marks one record, and the end reconciles the whole held set. The W3 figures in
  [decision 0019](0019-service-runtime-tokio-and-http.md#evidence-from-the-mvp) were
  measured with the ack removing its record and were not taken again.
- A publish onto a held key answers `superseded` though nothing is shed, as the
  maintainer chose at step 1, and an ack of a nacked record or a nack of an acked one is
  a new refusal, `MarkConflict`, 409 `mark_conflict` on the wire.
- The specification leaves the order of the entries one lease end creates open; the
  library pins one, smallest held position first
  ([Testing](../reference/testing.md#tests-from-the-specifications)).

## What would reopen this

A consumer that needs a mark to act before its lease ends: an operator who must see a
dead letter at the nack, or long leases whose acked records cost too much memory while
held. A bound on records, if question 3 on capacity is answered with one, that two
versions of a key would break. A durable store that cannot reconcile a held set at the
lease's end as one step. A need to retract a mark under the lease.

## Related pages

- [Specifications](../explanation/specifications.md)
- [Terminology](../project/terminology.md)
- [HTTP API](../reference/http-api.md)
- [Decision 0019: The service runtime: tokio, one owner task and HTTP/JSON](0019-service-runtime-tokio-and-http.md)
- [`pachinko.allium`](../specs/pachinko.allium), the root module
