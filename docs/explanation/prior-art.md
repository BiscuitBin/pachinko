---
title: "Prior art"
kind: "explanation"
audience: [contributor, maintainer, agent]
canonical_for: [prior_art]
requires: []
---

# Prior art

The root module's first open question asks for a decision "from the conflict-resolution
literature and from what comparable queues and databases do". This page records what a
survey of those queues and databases found, so that the maintainer can decide the open
questions, and whether to build or adopt, from evidence. The model the candidates were
compared with is the one [`pachinko.allium`](../specs/pachinko.allium) specifies and
[Purpose and scope](../project/purpose-and-scope.md) summarises.

Nothing found reproduces that model. Every piece of it exists somewhere, and the part no
other system was found to have is one rule: a record replaced according to a rank its
publisher supplies, inside a pocket held under a lease. The survey was made on 2026-11-12
from documentation and, in three cases, source code. No candidate was installed or run,
and every statement about a product is that product's documentation as read on that day.

## How to read the evidence

Each claim carries the grade of the reading behind it.

- **Three readers:** three independent verifiers voted on the claim against its source.
- **Second reader:** a separate reader, asked to refute the claim, re-read the cited
  source and the claim held.
- **One reader:** one reader fetched the official page and quoted it. Nobody checked the
  reading.
- **Source read:** added where the claim rests on source code and not on documentation.

Where no grade is given, the claim is one reader's. "Not found" means the pages read are
silent; it does not mean the feature is absent. A dash is a cell the survey left blank.

This page uses the project's words for this queue: pocket, record, rank, supersede,
lease, consumer and publisher ([Terminology](../project/terminology.md)). Another
system's description keeps that system's words, so a session, a message group or a job
appears where its documentation says so.

## Verdict

| Lens | Verdict | Nearest |
| --- | --- | --- |
| Same problem: one consumer at a time for each pocket, the newest state of each key wins, and pockets are handed out fairly | None as one product | BullMQ Pro and Hatchet reach all three, but their "newest" is the newest to arrive |
| Same outcomes | None | No queue found makes an older version that arrives late lose. Only stores do: Elasticsearch external versioning, DynamoDB conditional writes and PostgreSQL's `ON CONFLICT ... WHERE` |
| Same model: a lease on a whole pocket, one ranked record for each key inside it, and a newer version that waits out the lease | None | Azure Service Bus sessions for the lease; the Kubernetes client-go workqueue for deferring while held; `leasedkeyq`, an alpha in-process Python library, for a keyed, leased queue that updates a waiting value in place |

Every "keep the newest" mechanism found in a queue decides by arrival order. The verdict
covers the systems read and no others;
[What this page does not establish](#what-this-page-does-not-establish) lists the limits.

## The closest neighbours

The survey graded each candidate against six properties of the model and one of the
hand-out order:

- **Lease:** an exclusive, renewable lease on a whole pocket, pulled in several batches.
- **Rank:** one record for each key, superseded in place by publisher rank, with a lower
  rank answered stale.
- **Deferral:** a newer version waits while the older one is held.
- **Expiry:** a lapsed lease returns the pocket with its position kept, and repeated
  lapses park the pocket.
- **Retention:** superseded records and dead letters are kept for a TTL.
- **Surface:** acks that name a version, atomic batches, a write-ahead log and HTTP/JSON.
  The write-ahead log comes from the maintainer's working notes, which the survey took
  as part of its baseline; the root module does not specify one.
- **Fairness:** fairness between pockets when they are handed out.

"Arrival" marks a mechanism that decides by arrival order and compares no rank.

| Candidate | Lease | Rank | Deferral | Expiry | Retention | Surface | Fairness | Grade |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Azure Service Bus sessions | yes, mostly | no | not found | partial | partial | partial | not found | Second reader |
| BullMQ with BullMQ Pro groups | partial | partial (arrival) | yes (arrival) | not found for groups | partial | not found | yes | Second reader |
| Hatchet concurrency keys | partial | partial (arrival) | partial | not found | not found | not found | yes | Three readers and second reader for the strategies; one reader for the rest |
| client-go workqueue | partial | partial | partial | no | no | no | FIFO only | Three readers, second reader, source read |
| SQS FIFO message groups | partial | no | no | partial | partial | not assessed | no | Three readers, second reader |
| Elasticsearch external versioning | no | partial (the rank rule: yes) | no | no | – | no | – | Three readers, second reader |
| ActiveMQ Artemis last-value queue | no | partial (arrival; moves to the tail) | partial | – | – | – | – | Three readers, second reader, source read |
| PostgreSQL libraries (pgmq, Graphile Worker, pg-boss, Oban, River) | partial; in the paid editions of Oban and River | no, or partial (arrival) | no, or partial | partial, for each job, or not found | partial; not found for Graphile Worker | SQL, partial or no | yes for pgmq, partial for Oban Pro, otherwise not found | One reader |
| Temporal, one workflow for each entity | partial | no; left to workflow code | partial | no | – | no | partial | Three readers for part; one reader for the rest |
| `leasedkeyq` | partial (a lease for each key, not each pocket) | partial (in place, no rank) | configurable: update, reject or buffer | partial | no | no (in memory) | FIFO only | One reader |

The survey ranked five of these closest. The PostgreSQL row is taken apart under the
fifth.

### Azure Service Bus sessions

Service Bus has the same lease model. A receiver that accepts a session "holds an
exclusive lock on all messages with that session's session ID", can renew that lock, and
can ask for the next available session without naming it
([Message sessions](https://learn.microsoft.com/en-us/azure/service-bus-messaging/message-sessions),
[`AcceptNextSessionAsync`](https://learn.microsoft.com/en-us/dotnet/api/azure.messaging.servicebus.servicebusclient.acceptnextsessionasync)).
The default lock lasts one minute and the longest five.

It lacks any supersede. Duplicate detection is by `MessageId` and drops the later
message, so the first wins. Delivery counts and dead-lettering apply to one message:
after `MaxDeliveryCount` deliveries, ten by default, the message moves to the dead-letter
queue and the receiver carries on with the session, so nothing parks a failing session.
Dead letters have "no automatic cleanup". No deferral of a newer version, no statement
of fairness or of the order in which sessions are offered, and no REST API for sessions
were found. The second reading added caveats to "yes, mostly": receiving in batches
under one session lock is inherited from the receiver type and not stated outright;
takeover after the lock expires is implied; exclusivity is the default and not the only
mode; and a receiver may name the session it wants.

Reproducing the model on it would take a versioned store beside the broker holding one
record for each key, with the session carrying only "this pocket has work". That is two
systems to keep consistent, with the rank rule outside the broker.

### BullMQ with BullMQ Pro groups

BullMQ is the nearest on outcomes: one job at a time for each group, round-robin between
groups, and the newest waits while one is active. The open-source `keepLastIfActive`
deduplication stores a new job's data while a job with the same ID is active and, when
that job finishes, creates a job from the latest stored data
([Deduplication](https://docs.bullmq.io/guide/jobs/deduplication)). Pro groups are
processed "in a 'round-robin' fashion", with a concurrency setting for each group
([Groups](https://docs.bullmq.io/bullmq-pro/groups)).

It lacks a rank: deduplication goes by arrival with no version comparison, so an older
version that arrives late wins. No lease held across pulls, no automatic parking of a
group and no dead-letter queue were found on the documentation pages read; the marketing
page lists dead-letter queues, and no documentation page was found behind that. The
second reading confirmed both quotations and found three points implied and not stated:
that the active job keeps running, that deduplication is scoped to the queue and not the
group, and that a group's concurrency may be one. BullMQ Pro is a commercial licence;
the pricing page showed the Standard plan at $1,395 a year for each deployment. BullMQ
runs on Redis, on compatible stores, and on PostgreSQL; whether Pro groups work on
PostgreSQL is not stated.

Reproducing the model on it would take deduplication IDs composed from the pocket and the
record key, and a rank check in the worker against an outside store.

### Hatchet

Hatchet covers the same ground as BullMQ Pro, with thinner documentation of a key for
each record. `GROUP_ROUND_ROBIN` gives fairness between groups, and
`CANCEL_QUEUED_EXCEPT_NEWEST` keeps the most recently queued run for a key, by arrival
([Concurrency](https://docs.hatchet.run/v1/concurrency)). A caller may set a priority
from one to three, which orders pickup and is not said to decide which run is cancelled.
It lacks what BullMQ lacks. Combining a key for the pocket with a record key is
undocumented beyond one round-robin example, so whether the combination works is not
known.

### client-go workqueue

The Kubernetes controller workqueue has the same deferral logic, in one process and in
memory. Adding an item that is being processed marks it dirty and does not queue it, and
`Done` queues it again if it is dirty
([`queue.go`](https://github.com/kubernetes/client-go/blob/master/util/workqueue/queue.go)).
It has no lease, timeout, payload, version or persistence, so an item whose worker never
calls `Done` is never handed out again. A workqueue in the Go project Rune, published in
2026, has the same state machine (one reader). Everything else in the model would be
new work on top of either.

### PostgreSQL, hand-built

Every primitive exists and is documented: `FOR UPDATE SKIP LOCKED`, advisory locks, and
`ON CONFLICT DO UPDATE ... WHERE` for an insert that replaces only on a comparison
([`SELECT`](https://www.postgresql.org/docs/current/sql-select.html),
[Explicit locking](https://www.postgresql.org/docs/current/explicit-locking.html),
[`INSERT`](https://www.postgresql.org/docs/current/sql-insert.html)). No write-up or
library was found that combines a claim on a whole pocket, a ranked replacement and a
fair order between pockets. The libraries each hold one or two pieces (all one reader):

| Library | Lease | Rank | Deferral | Expiry | Retention | Surface | Fairness |
| --- | --- | --- | --- | --- | --- | --- | --- |
| pgmq grouped FIFO reads | partial | no | no | partial | partial | SQL only | yes |
| Graphile Worker | partial | partial (arrival) | partial | not found | not found | SQL only | not found |
| pg-boss | partial | partial (arrival) | partial | partial | partial | partial | not found |
| Oban and Oban Pro | partial (Pro) | partial (arrival) | no | for each job | partial | no | partial (Pro) |
| River and River Pro | partial (Pro) | no | partial (Pro) | partial (Pro) | partial | no | not found |

pgmq has grouped reads that skip a group whose head is in flight, and a round-robin read
([FIFO queues](https://pgmq.github.io/pgmq/latest/fifo-queues/)). Graphile Worker
replaces a waiting job by `job_key` and, when that job is running, schedules a second
job and leaves the running one alone
([Job key](https://worker.graphile.org/docs/job-key)). pg-boss has a policy of one
queued and one active job for a key ([Queues](https://pgboss.io/api/queues)). River Pro
sequences run a key's jobs one at a time and halt on a discarded job
([Sequences](https://riverqueue.com/docs/pro/sequences)).

## Build or adopt

Nothing found can be adopted as it stands. Each path below leaves the rank rule, the held
record with its deferred successor, and the parking of a whole pocket to be built.

| Path | What it gives | What is still to build |
| --- | --- | --- |
| PostgreSQL tables | Durability, transactions, atomic ack batches, the replace-on-comparison insert | A sweeper for lapsed leases, the held and deferred states, parking, the decay of retained records, the HTTP service |
| Service Bus sessions with a versioned store | The lease, recovery, dead-lettering of single messages | Rank, deferral, parking, fairness, consistency between the two systems; it runs on Azure only |
| BullMQ Pro | Exclusion, fairness, deferral of the newest by arrival | Rank, the lease's behaviour, parking; it needs a commercial licence and Redis or PostgreSQL |
| Temporal, one workflow for each pocket | Exclusion, durable state, fairness keys | All of the queue's logic as workflow code; a limit of 10,000 signals on a workflow's history forces Continue-As-New ([Limits](https://docs.temporal.io/workflow-execution/limits)) |

The PostgreSQL row is an inference from the documented primitives, not a finding: no
reader produced a costed design and nothing was built. On that inference it is the one
path of the four that would keep the model's rules in a single system, and it would put
a database to operate where the maintainer's working notes plan a write-ahead log. This
page does not weigh the paths against building the service as the root module has it.

## What is distinct

The survey's statement of what sets the model apart, in this project's words:

> A queue where the unit of lease is a whole pocket and the unit of storage is one
> publisher-ranked record for each key: a lower rank loses at publish time even if it
> arrives last, and a higher one waits out the lease instead of disturbing it.

Against each family of neighbours, that is:

- **Session and message-group queues** (Service Bus sessions, SQS FIFO): it adds
  supersession by rank, so a backlog collapses to the newest state of each key.
- **Last-value and compaction systems** (Artemis, NATS JetStream, Kafka, EMQX): rank in
  place of arrival, a kept position, and exclusion for each pocket.
- **Job engines with concurrency keys** (BullMQ Pro, Hatchet, Inngest, Oban Pro): a lease
  the consumer holds across pulls in place of a cap on dispatch, and rank.
- **The controller workqueue:** the same idea of deferring a repeat while its key is
  held, as a network service with payloads, ranks and leases.

The statement is supported against the systems this page and the survey's notes list,
and against no others.

## Evidence for the open questions

The root module carries eight open questions. Each section below lists what comparable
systems do, in the module's order, and ends with what was not found. None of it answers
a question: the questions stay the maintainer's, as
[Specifications](specifications.md) explains. Items are one reader's unless marked.

### Equal rank, different payload

The question lists four options, and each has a precedent.

- **Reject.** Elasticsearch's `external` version type accepts a write only when its
  version is strictly higher than the stored one, and otherwise answers HTTP 409
  ([Index API](https://www.elastic.co/docs/api/doc/elasticsearch/operation/operation-index);
  three readers, second reader). A DynamoDB conditional write that checks the version is
  equal lets the first commit win
  ([Working with items](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/WorkingWithItems.html)).
- **Last arrival wins.** Elasticsearch's `external_gte` also accepts an equal version;
  the reference says it "should be used with care", and the 8.19 guide that it "can
  result in loss of data" (three readers, second reader). Kafka's KIP-280 proposal
  breaks a tie by the highest offset; the proposal is marked accepted and its page names
  no release (no address was recorded for it). In a Kafka Streams versioned state store,
  a second put with the same key and timestamp replaces the first
  ([KIP-889](https://cwiki.apache.org/confluence/display/KAFKA/KIP-889%3A+Versioned+State+Stores)).
- **Keep both.** Riak keeps siblings when `allow_mult` is true
  ([Conflict resolution](https://docs.riak.com/riak/kv/latest/developing/usage/conflict-resolution/);
  from a summarised tail of the page). The multi-value register of the CRDT report below
  retains both values.
- **Configurable.** Elasticsearch chooses the version type on each request, and BullMQ
  the deduplication mode on each job.
- **First wins** is the default of every deduplication feature read: SQS (second
  reader), Service Bus, Cloud Tasks, River, Sidekiq and JetStream's `Nats-Msg-Id`.

Two sources follow a fifth pattern, which the question does not list: a tie is either
ruled out or settled by a rule that ignores arrival.

- **Unique ranks.** The last-writer-wins register in Shapiro and others' CRDT report
  (RR-7506, section 3.2.1) assumes the tie away: "Timestamps are assumed unique, totally
  ordered, and consistent with causal order", for example a counter for each replica
  joined to a unique replica identifier. It gives no tie-break, and its two
  specifications differ on a tie: the state-based merge lets the incoming value win and
  the operation-based one ignores it. The report was read from
  [a mirror](https://ondoc.logand.com/d/8218/txt), because the publisher's host refuses
  automated readers.
- **A deterministic rule on the payload.** Cassandra settles equal timestamps without
  regard to arrival: a tombstone or an expiring cell beats a live one, and between two
  live cells the greater value wins
  ([`Cells.java`](https://raw.githubusercontent.com/apache/cassandra/cassandra-5.0/src/java/org/apache/cassandra/db/rows/Cells.java),
  source read on 4.1 and 5.0). Its user documentation only warns that ties can reorder
  a batch. Under such a rule the outcome is the same in whatever order the writes
  arrive or are replayed.

Not found: a queue's answer to an equal rank. Every keep-the-newest mechanism found in
a queue decides by arrival order, so the precedents above come from stores. What users of these systems expect, which the question also asks for,
was not surveyed.

### Reading superseded and dead-letter records

- **A listing with a redrive an operator starts.** SQS moves messages from a dead-letter
  queue back to the source queue on request
  ([Dead-letter queue redrive](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-configure-dead-letter-queue-redrive.html)).
  Service Bus resubmits from the portal's explorer, and has "no automatic cleanup".
  Sidekiq has a Dead tab with a manual retry; River Pro a dead-letter table retried
  through an API; pg-boss a `redrive()` call.
- **A consumable stream.** RabbitMQ's dead-letter exchanges are ordinary exchanges
  ([Dead letter exchanges](https://www.rabbitmq.com/docs/dlx)). A Pub/Sub dead-letter
  topic needs its own subscription. Confluent share groups write exhausted records to a
  dead-letter topic.
- **Replay in bulk by a filter.** Inngest and Trigger.dev replay runs chosen by status
  and time range.
- **An advisory only.** JetStream has "no built-in dead-letter queue" and publishes an
  advisory.

Not found: anything that keeps superseded versions as a readable set with a TTL. The
nearest are incidental: Kafka's older versions linger until the log cleaner runs, and
Inngest leaves a deduplicated event visible in its event history.

### Capacity

- **Reject the publish when full.** RabbitMQ's `reject-publish` overflow setting
  ([Queue length limit](https://www.rabbitmq.com/docs/maxlength)); JetStream's
  discard-new policy; BullMQ Pro's `maxSize`, which throws; Trigger.dev's queue size
  limit, which rejects new triggers.
- **Drop the oldest.** RabbitMQ's `drop-head` and JetStream's discard-old, each the
  default.
- **A bound by count and by age together.** Sidekiq's dead set holds 10,000 jobs or six
  months.

Every broker read offers a way to reject a publish for capacity, and two drop the oldest
message by default. The root module's `NeverRefusedForCapacity` guarantee sheds retained
records and never a record. Not found: anything on the order in which retained records
are shed, or on spilling to disk; the survey recorded nothing on either.

### Routing by worker pool or priority

Everything read uses separate queues together with a priority field. Temporal dispatches
a higher priority before it applies fairness
([Task queue priority and fairness](https://docs.temporal.io/develop/task-queue-priority-fairness)).
RabbitMQ has consumer priority. Trigger.dev sets a machine preset on each task.

Not found: a system that makes the worker pool a property of a group.

### Adaptive lease TTL

One direct precedent was found. Google Pub/Sub's Python, Go and Java client libraries
"use the 99th percentile of acknowledgment delay to determine the length of each
extension", within configured bounds
([Lease management](https://docs.cloud.google.com/pubsub/docs/lease-management)). The
adaptation is in the client, not the service. SQS, Service Bus and Kafka share groups
document a fixed timeout with an explicit renewal; Oban, River and Hatchet document
fixed timeouts.

Not found: a service, among those read, that derives its own timeout from the durations
it observes.

### A new pocket jumping the queue

FQ-CoDel puts a new queue on a separate list that is served first, and moves a new queue
that empties to the end of the old list before removing it, a step the specification
calls "crucial to prevent starvation"
([RFC 8290](https://www.rfc-editor.org/rfc/rfc8290.html), read in full). SQS fair queues
deliver a quiet tenant's messages ahead of a noisy one's
([Fair queues](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-fair-queues.html);
three readers), on standard queues only.

Not found: any job engine that documents it. The Deficit Round Robin paper was not read.

### Reviving a parked pocket

- **An operator's action.** River Pro sequences halt on a discarded job and resume when
  an operator retries it. BullMQ Pro and Trigger.dev pause and resume a group or a key.
  Sidekiq's poison pills, jobs recovered three times in 72 hours, are retried from its
  web interface.
- **A cooldown.** None was found for a group. Backoff lists for single messages exist,
  in JetStream and in RabbitMQ's delayed retry.

Parking a whole group automatically was itself found in two places only: River Pro's
sequence halt, and pg-boss's `key_strict_fifo` policy, which "holds back successors
while a job with the same key is active, in retry, or failed".

Not found: a parked group revived by a timer.

### A key re-published within the lease

- **Treated as new work.** The client-go workqueue (three readers, second reader).
  Sidekiq and Faktory release the unique lock on success. BullMQ ignores a duplicate
  only until the job completes.
- **Blocked for a period.** Oban's default uniqueness counts completed jobs
  ([Unique jobs](https://hexdocs.pm/oban/unique_jobs.html)). River blocks until the
  completed job is pruned. Trigger.dev keeps an idempotency key for 30 days.

For the neighbouring case, a publish while the key is still held, `leasedkeyq` makes it
a choice on each call between updating the held value, rejecting, and buffering a second
copy ([`leasedkeyq`](https://github.com/nickweedon/leasedkeyq)).

Not found: anything that compares the new publish with the version just settled.

## What this page does not establish

- **Most rows had one reader.** Every claim graded one reader is a single reading of
  official pages. Several long pages reached the reader cut short and partly summarised:
  Pulsar's Key_Shared description, KIP-932, Confluent's share-group page, the
  Elasticsearch API reference and Riak's conflict-resolution page. The claims with three
  readers covered nine candidates. The second reader re-read 13 claims: ten held as
  written, three needed the qualifications given above and none was refuted.
- **Three things need a test and not more reading:** whether Hatchet's combination of
  two keys works; whether BullMQ's `keepLastIfActive` with composed IDs and Pro groups
  behaves as one record for each key in each pocket; and in what order Service Bus
  offers its "next available" session.
- **The Deficit Round Robin paper was not read** in its primary form. Six hosts were
  tried and none gave readable text. What a secondary source says of it does not cover
  where a newly active flow is placed, so RFC 8290 is the only primary source here for
  the question of a new pocket jumping the queue. The Kafka-compatible forks were not
  read either.
- **The candidates listed are not all that exist.** The survey read more than thirty
  systems and this page carries those that bear on a decision. A system absent from both was
  not assessed.
- **Nothing was run.** Behaviour under load, at a failure or at the edge of a documented
  limit is as the documentation states it, on 2026-11-12.

## Related pages

- [Specifications](specifications.md)
- [Purpose and scope](../project/purpose-and-scope.md)
- [Terminology](../project/terminology.md)
- [Decision 0019](../decisions/0019-service-runtime-tokio-and-http.md), which closed the
  one open question that is no longer in the module
- [`pachinko.allium`](../specs/pachinko.allium), the root module
