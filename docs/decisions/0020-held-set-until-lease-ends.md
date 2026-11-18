---
title: "Decision 0020: A held record stays held until its lease ends"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_held_set]
requires: []
---

# Decision 0020: A held record stays held until its lease ends

## Context

The specification states this decision: `docs/specs/pachinko.allium` holds it in the
Publish comment, `DeferBehindHeldVersion`, the ack and nack rules, the "Lease end" rules
and the `Consuming` guarantees `HeldSetStableUntilLeaseEnds`, `MarksStandHoweverLeaseEnds`
and `NothingLostWhileHeld`. The maintainer asked on 2026-11-16, at step 1 of ticket T35,
for this terse record of why as well.

Under the rules before it, an ack or a nack removed the held record at once. The wave's
code review found one version shed twice (open question 10), and the module already
asked whether one lease could be handed a key twice (open question 9):

![Two timelines under the rules before this decision. Open question 10: consumer c1 pulls k at rank 1; a publish of rank 2 makes Supersede shed rank 1, which c1 still holds, into the superseded queue; c1's nack then makes rank 1 a dead letter too. Open question 9: under lease L, c1 pulls k at rank 1 and acks it, which removes the record; a publish of rank 2 creates a new, queued record; c1 pulls again under L and is handed k a second time.](0020/defect.svg)

## Decision

The maintainer's, in an elicitation on 2026-11-13, recorded in ticket T35. A held record
keeps the rank and payload it was handed out with until its lease ends; an ack or a nack
only marks it:

![A record is queued until a pull under a lease makes it held. While held it is unsettled until an ack marks it acked or a nack marks it nacked; the first mark stands, a repeat only extends the lease, and the other mark is refused with MarkConflict. A publish above the stored rank waits behind the held version, and a still higher one replaces only the waiting version; nothing held changes. The lease ends by release or by lapse: a record that is unsettled, or has a newer version deferred behind it, is queued again; an acked or nacked record with nothing deferred is removed.](0020/held-record.svg)

The lease's end reconciles every held record by its mark. This copies the specification's
"Lease end" table; where they differ the specification wins.

![When a lease ends, each held record goes by its mark and by whether a newer version is deferred behind it. Unsettled, nothing deferred: the held version goes nowhere and the record is queued at the same position (RequeueUnsettledRecord). Unsettled, a newer version deferred: the held version goes to the superseded queue and the newer version is queued at the held version's position (SupersedeUnsettledHeldVersion). Acked, nothing deferred: consumed, and the record is removed (ConsumeAckedRecord). Acked, a newer version deferred: consumed, and the newer version is queued at its arrival position (ConsumeAckedVersionQueueDeferred). Nacked, nothing deferred: a dead letter, and the record is removed (DeadLetterNackedRecord). Nacked, a newer version deferred: a dead letter, and the newer version is queued at its arrival position (DeadLetterNackedVersionQueueDeferred).](0020/lease-end.svg)

The lease keeps its name; the properties are stated as guarantees rather than by calling
it a transaction.

Why:

- **The consumer works on a stable set.** No publish changes or removes a version the
  consumer holds, and no key is handed out twice under one lease. The lease read shows
  the set as it was handed out, a snapshot only the consumer's own pulls and marks change.
- **A mark is committed when it is processed, however the lease ends.** A lapse keeps
  every ack and nack, so work a consumer finished before it died is not redone, and a
  repeated mark is safe to retry. Acked means consumed: an acked version is recorded
  nowhere.
- **No double shed.** Nothing held is shed or dead-lettered while the lease is active, so
  each version that leaves ends in exactly one place: the superseded queue if no consumer
  acked or nacked it, the dead letters if it was nacked.
- **One rule judges a publish.** The kept record's rank decides, whether that record is
  unsettled, acked or nacked, which closed question 9.

Total fairness of positions, the maintainer's own note: a key no consumer acked or nacked
keeps its place in the pocket's order, and behind an acked or nacked version the newer
version is a new arrival, queued where it arrived.

![A pocket's records by position. The lease holds k at rank 1 at position 11; b and c are queued at 12 and 13; k at rank 2 then arrives, is deferred and is given arrival position 14. If k rank 1 is unsettled when the lease ends, it goes to the superseded queue and k rank 2 is queued at 11, its place. If k rank 1 was acked or nacked, it is consumed or becomes a dead letter, and k rank 2 is queued at 14, where it arrived, behind b and c.](0020/positions.svg)

## Consequences

- A pocket may hold two versions of a key: the held one and the highest that arrived
  since. The store keeps both, with the mark, the nack's time and the arrival position.
- Dead letters appear only when the lease ends, and an unsettled held version a newer
  one replaces is recorded as superseded then, not at the publish. A consumer that pulls
  in a loop and never releases keeps its acked records stored and on the lease read, and
  holds back its dead letters, until it releases or the lease lapses.
- The specification leaves the order of the entries one lease end creates open; the
  library pins one, smallest held position first
  ([Testing](../reference/testing.md#tests-from-the-specifications)).

The lease-end work moved from the ack to the release and the lapse:

```diff
  an ack
- removes the record, or drops the held version and defers the newer one to the lease's end
+ marks the record acked
  a nack
- dead-letters the held version, then removes the record or defers the newer one to the lease's end
+ marks the record nacked, with the nack's time
  a release or a lapse
- requeues the records still held or deferred
+ reconciles every held record by its mark: the six outcomes above
```

The W3 figures in [decision 0019](0019-service-runtime-tokio-and-http.md#evidence-from-the-mvp)
were measured with the ack removing its record and were not taken again.

A publish onto a held key is judged against the stored rank, which is the newest
version's, not the held one's:

| The publish's rank against the stored rank | Rule | Answer | What changes |
| --- | --- | --- | --- |
| higher, nothing deferred | `DeferBehindHeldVersion` | `superseded` | The new version is deferred behind the held one at the next arrival position. Nothing is shed; the maintainer chose `superseded` for it at step 1. |
| higher, a version already deferred | `Supersede` | `superseded` | The deferred version is shed to the superseded queue at once; the new one takes its place and its arrival position. |
| equal | none | `equal_rank`, a 409 on the single `PUT` | Nothing. |
| lower | `DropStalePublish` | `stale` | The published version becomes a superseded record, reason stale. The pocket is not touched. |

An ack of a nacked record or a nack of an acked one is a new refusal, `MarkConflict`,
409 `mark_conflict` on the wire.

## What would reopen this

- A consumer that needs a mark to act before its lease ends: an operator who must see a
  dead letter at the nack, or long leases whose acked records cost too much memory while
  held.
- A bound on records, if question 3 on capacity is answered with one, that two versions
  of a key would break.
- A durable store that cannot reconcile a held set at the lease's end as one step.
- A need to retract a mark under the lease.

## Related pages

- [Specifications](../explanation/specifications.md)
- [Terminology](../project/terminology.md)
- [HTTP API](../reference/http-api.md)
- [Decision 0019: The service runtime: tokio, one owner task and HTTP/JSON](0019-service-runtime-tokio-and-http.md)
- [`pachinko.allium`](../specs/pachinko.allium), the root module
