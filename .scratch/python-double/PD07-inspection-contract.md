---
id: PD07
title: "Python double: is the inspection shape part of the contract?"
status: done
depends_on: [PD06]
parallel_with: [PD08]
branch: ticket/pd07-inspection-contract
estimated_size: S
---

# PD07: Python double: is the inspection shape part of the contract?

## Context

Two Rust tests the series ported assert facts about the Rust's inspection snapshot that
`docs/specs/pachinko.allium` does not state, and a double that follows the specification
fails them when they are ported literally:

- **S053** (PD03, spec note N24). The Rust's test asserts that an ended lease leaves the
  registry: its snapshot's lease list is empty. The specification's `Lease` ends
  `released` or `expired`, both terminal, with `ended_at`, and no rule removes it. The
  double keeps the ended lease and ports the specification's facts instead.
- **S091** (PD04, spec note N31). The Rust's test lists a pocket's records in its own
  snapshot order, `[("b", 3), ("z", 1)]`. The specification treats a pocket's records as
  a set, and no rule asks them to be listed. The double keeps insertion order, so it
  lists `[("z", 1), ("b", 3)]`. The test compares positions by key.
- **S066, S093 and S075** (PD05, spec note N38). PD05 ported three more rows under the
  same reasoning, and the maintainer accepted them and added them to this ticket's scope.
  S066's release half asserts that the released lease leaves the Rust's registry, as
  S053 does. S093 lists a pocket's records in the Rust's snapshot order, as S091 does.
  S075 checks, at each step, that the Rust's stored pocket order is the queued pockets
  sorted by position. The double stores no pocket order, because the specification
  derives it, so the test asserts the position invariants and an empty `check` instead.

The maintainer accepted these ports for now (PD03's, PD04's and PD05's hand-backs
record the rulings) and asked for this follow-up, to investigate whether the
inspection shape belongs in the contract.

## Goal

A written answer, for the maintainer to accept or reject, to three questions:

1. Is an ended lease part of the observable state, or is the registry of leases an
   implementation detail? What does fencing by lease id need kept, and for how long?
2. Is the order in which an inspection lists a pocket's records part of the contract? If
   so, which order: by key, by position, or another?
3. Is the order of the queued pockets something an implementation stores and an
   inspection shows, or only what the specification derives from their positions
   (S075)?

## Non-goals

- No change under `docs/specs/`. A finding becomes a proposal for the maintainer, who
  moves the specification first if they accept it (`spec-change`).
- No change under `crates/`.
- No change to the double's behaviour until the maintainer rules.

## Steps

1. Read N24, N31 and N38 in `double/SPEC-NOTES.md`, the three hand-backs, the specification's
   `Lease` entity and the inspection surface, and the catalog rows S053, S066, S075, S091 and S093.
2. List every other ported test whose assertion depends on the inspection shape: an
   order, the presence of an ended entity, or a derived set. Use
   `rg -n 'impl-choice|N24|N31|N38' double/tests`.
3. For each question, write the options with their consequences for the Rust, the double
   and the catalog. Recommend one.
4. Put the recommendation to the maintainer. Stop.

## Acceptance criteria

- All three questions have a recommendation, with the evidence from step 2.
- The maintainer's answer is recorded in the hand-back notes.

## Verification

```sh
set -eu
git rev-parse HEAD
just python-double
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

### 1. Base and branch

- Branch: `build-python-double`.
- Base commit: `7b6bfd1259ca3da673d17f2586671d65781627d1`, PD08's ticket commit.
- Work commit: `fe400fa34acdc319812d699aa22b3d16d988617f`.
- The ticket names `ticket/pd07-inspection-contract` in its own worktree. It ran on this
  branch in this worktree instead, as PD01 to PD06 did at the maintainer's direction, and
  as the maintainer's `/implement` asked ("commit your work to the current branch").
  `ticket/pd07-inspection-contract` was not created.
- `just initialize` was not run. The worktree's baseline was green without it.

### 2. Baseline

Both ran on the untouched worktree at `7b6bfd1259ca3da673d17f2586671d65781627d1`
(`ai_tmp/pd/PD07/baseline-python-double.txt`, `ai_tmp/pd/PD07/baseline-check.txt`).

- `just python-double` printed `Ran 274 tests` and `OK` (exit 0).
- `just check` ended `All checks passed and the worktree is unchanged.` (exit 0).

### 3. Step 2: the tests that depend on the inspection shape

#### What the grep finds

`rg -n 'impl-choice|N24|N31|N38' double/tests` prints 23 lines. Five concern the
inspection shape this ticket asks about:

| Line | Test | Scenario | What it leans on |
| --- | --- | --- | --- |
| `test_settle.py:121` | the helper `ended_lease` | S053, S055, S057 | The ended lease is in the snapshot (N24) |
| `test_expiry.py:201` | `test_a_release_after_expires_at_but_before_the_lapse_succeeds` | S066, release half | The released lease is in the snapshot (N24) |
| `test_deferral.py:398` | `test_the_entries_one_lease_end_creates_come_out_smallest_held_position_first` | S091 | Compares the kept positions by key (N31) |
| `test_endings.py:290` | `test_at_an_expiry_limit_of_one_a_marked_version...` | S093 | Compares the records by key (N31) |
| `test_expiry.py:612` | `test_pockets_and_records_form_one_strict...` | S075 | Asserts the position invariants and `check`; the Rust's stored-order check has no counterpart (N38) |

Two more are sibling orders, which finding 7 already holds: `test_ack_nack.py:252`
(S090, the lease view's order, N23) and `test_deferral.py:360` (S091, the order of the
entries one lease end creates, N29). The other 16 lines concern notes that say nothing of
the inspection shape (N04, N09, N10, N14, N16, N18, N22 and N26).

The grep misses most of the dependence, because a test that reads an ended lease or a
listing does not have to cite a note. The rest of this section goes past it.

#### Measured: which tests stop passing under each shape

A scratch runner, `ai_tmp/pd/PD07/probe.py`, imports the double, wraps the snapshot in
memory for one hypothetical shape, and runs the discovery `just python-double` runs. It
writes nothing in the tree. Its output is in `ai_tmp/pd/PD07/probe.txt`, headed by
`7b6bfd1259ca3da673d17f2586671d65781627d1`, with uncommitted changes to this ticket's
file only. Each case ran all 274 tests and exited 0. The
counts are test functions that did not pass, not counting the coverage test's own
knock-on failures (it fails whenever another test does).

| Case | What changes | Tagged ports | Untagged tests |
| --- | --- | --- | --- |
| `control` | Nothing | 0 | 0 |
| `forget-ended` | `Queue.snapshot()` leaves out every lease that is not active | 11 | 1 |
| `forget-ended-in-state` | The same, and those leases are also deleted from the queue's state before each apply step, so no rule's view holds one | 11 | 1 |
| `records-by-key` | `Queue.snapshot()` lists records by pocket key, then record key | 0 | 0 |
| `records-by-position` | `Queue.snapshot()` lists records by pocket key, then position | 0 | 0 |
| `all-records-by-key` | Every `Snapshot` does, the rules' views and hand-built ones included | 0 | 2 |
| `all-records-by-position` | The same, by position | 0 | 0 |
| `state-records-reversed` | Every snapshot a `Queue` makes lists records in reverse, the rules' views included | 0 | 1 |
| `state-leases-reversed` | The same for leases | 0 | 1 |
| `shed-reversed` | `Queue.snapshot()` lists the superseded records and the dead letters in reverse | 8 | 0 |
| `joins-reversed` | `Queue.snapshot()` lists the publishers and the consumers in reverse | 2 | 0 |
| `pockets-reversed` | `Queue.snapshot()` lists the pockets in reverse | 0 | 1 |

What each row found:

- **`forget-ended`.** The 11 tagged ports are S053, S055 and S057 (`test_settle.py:122`,
  through `ended_lease`), S059 and S066's release half (`test_expiry.py:133` and `:209`),
  and S078, S079, S080, S081, S087 and S088 (`test_endings.py:90` and `:137`). The
  untagged one is `test_guide`: the README's walkthrough prints each ended lease.
- **`forget-ended-in-state`.** The same 12 and no other. This is the case that tests
  fencing. A rule's view is built from the queue's state (`queue.py:385`), not from
  `Queue.snapshot()`, so only here does a fencing guard meet a lease that is really gone.
  S052, S068, S069, S070, S066's renew half and S067's ack half all passed. S057's second
  release is reached in neither run, because the test stops earlier, at its read of the
  ended lease (`test_settle.py:183`); S068 covers a release that names a forgotten lease.
- **A pocket's records, by key or by position.** No tagged port depends on the order.
  With the sort inside `Snapshot` itself, two hand-built tests fail under key order
  only: `test_a_pockets_records_split_into_queued_and_held` (`test_inspection.py:81`) and
  `test_check_names_every_violation_with_a_witness_each` (`test_invariants.py:74`). Both
  build a snapshot by hand and read it back in the order they wrote it.
- **`state-records-reversed`.** This is the strongest of the record cases: every rule's
  view lists records backwards. All 32 rules still fired and every scenario still
  passed, so no rule's behaviour reads the listing order. It is one permutation over the
  existing tests, not a proof. The one failure,
  `test_positions_come_from_next_position_in_allotment_order` (`test_queue.py:167`), reads
  the flat list of all records across two pockets, not one pocket's records.
- **`pockets-reversed`.** No tagged port. The one failure is the same test, which reads
  the flat list of pockets too.
- **`state-leases-reversed`.** One failure,
  `test_one_tick_lapses_in_expires_at_order_then_grant_order` (`test_decay.py:189`). The
  tick breaks a tie on `expires_at` by the order the leases are listed in (N34). That is
  a listing order that reaches behaviour. It is not one of this ticket's three questions
  and stays with finding 7.
- **`shed-reversed`.** S015, S018, S019, S073, S074, S082, S089 and S091 compare the
  superseded records or the dead letters in the order they were created. That order is
  finding 7's (N25, N29 and N34). The Rust lists both queues "by expiry and arrival"
  (`crates/libpachinko/src/queue/inspect.rs:84-87`). Whether the two orders can differ
  was not tested.
- **`joins-reversed`.** S009 and S010 compare the publishers and the consumers in join
  order (`test_identity.py:22` and `:34`). The Rust lists both by id. Its S009 compares
  the list and passes because the ids sort in join order; its S010 reads no order
  (`crates/libpachinko/tests/queue/identity.rs:103` and `:125`). No finding holds this
  one; it is small, and the same kind of thing.

The pocket order has no case of its own. The double stores none, so there is nothing to
vary: `test_it_holds_the_whole_state` (`test_inspection.py:23`) pins the snapshot's nine
fields, and none is an order. The tests derive the order instead, with `pocket_order`
(`test_publish.py:40`), which sorts the queued pockets by position; it is called at 16
sites.

#### Read, not measured

These come from reading the tests, with no run of the runner behind them:

- **A later forgetting.** S058 (`test_expiry.py:285`) and S065 (`test_expiry.py:136`)
  compare whole snapshots across later ticks while an ended lease is in the state. They
  pass when the lease is gone at once, as both runs above show, and would fail only if a
  retention rule dropped it during those ticks.
- **The ended lease through an answer.** S006, S061, S062, S089 and S093 read the lapsed
  lease from `TickReport.lapsed`, not from the snapshot. The Rust's tick report carries
  it too.
- **The lease view's held records.** S036, S085 and S090 compare them in order, smallest
  position first (N23). That listing is on the `Consuming` surface, not the inspection.

### 4. The answer

The three questions are answered as the ticket asks them. The answer does not claim that
no inspection shape is contract. Two orders reach a caller or behaviour, and both stay
with finding 7: the lease read lists the held records smallest position first, over HTTP
too (N23, `docs/reference/http-api.md:62`), and a tick breaks ties by the order leases
are listed in (N34).

It also keeps two contracts apart. One is the specification's: its rules, invariants and
two surfaces. The other is libpachinko's `testing` module, a public API behind a feature
that no normal build turns on. Its field docs do promise Rust test authors "Its records,
by key" and "An ended lease leaves the registry"
(`crates/libpachinko/src/queue/inspect.rs:49` and `:82`). Those promises are the Rust's
to keep or change. The questions here are about the first contract only.

In short: all three shapes are the implementation's, not the specification's. Nothing in
the Rust, the double or the catalog has to change, and the five ports stand. Each answer
below also says what it concedes, because for the first question the specification's
letter is against it.

#### Question 1: is an ended lease observable state?

**What the specification says.** A lease ends `released` or `expired`, both terminal,
with `ended_at` (`pachinko.allium:249-270`). Read literally, the specification keeps
every ended lease for ever. Its only `ensures: not exists` clauses remove a record (847,
887), a superseded record (924) and a dead letter (929), and every lease lookup filters
on `status: active` (493, 507, 526, 547), which it would not need to do if ended leases
were gone. Nothing in it says an ended lease may be forgotten.

What it reads of an ended lease is little:

- No surface shows one. `Consuming` exposes `Leases where consumer = consumer and status
  = active` (1115), and `Publishing` exposes nothing.
- No entity has a field that names a lease. A dead letter copies the consumer, not the
  lease (240). `Consumer` and `Pocket` each have a `leases` relationship that includes
  ended leases (114, 133), and it is read only through `active_leases` (115, 134).
- `ended_at` is read by `EndedAfterGrant` (267-269) and by nothing else. `Freed` carries
  the time as its own parameter (790-792).
- The six `Freed` rules read `lease.pocket` and `lease.consumer` (798-918), inside the
  outcome that ends the lease.
- After that outcome, three things still read an ended lease, and each reads only
  whether it is active. An action that names it meets `requires: lease.status = active`
  (579, 593, 605, 620, 639, 646, 668). `LeaseLapses` fires on a released lease when its
  `expires_at` passes, and its own `requires` stops it (699-703; this is S058). And two
  invariants range over every lease: `GrantedBeforeExpiry` (263-265) and
  `OneActiveLeasePerPocket` (997-1002). Each comes out the same when the lease is absent.

**What fencing needs.** The specification fences by the lease itself: ack, nack, renew
and release take the lease and require `lease.status = active` (554-557, and the
guarantee `ActionsNameTheirLease`, 1140-1144). The lease has no id (finding 3), so in the
model the entity is the token, and a model's entities are never forgotten.

An implementation swaps the entity for an id. Then fencing needs one thing kept, for the
life of the queue: **no id is ever issued twice**. It needs nothing of the ended lease,
for any length of time.

- The Rust does exactly this: "an ended lease is unknown rather than remembered, one
  refusal covers a lease that ended and one that never existed ... because the new lease
  has a new id" (`docs/explanation/architecture.md:223-227`).
- The double keeps the ended lease, but its fencing does not use it. `_acting_lease`
  returns None for an id that is absent, ended or another consumer's alike
  (`double/pachinko_double/settle.py:62-77`; renew's guard repeats the three tests,
  `:275-279`), and each refusal is `LeaseFenced` (`:119-120`, `:224-225`, `:306`). The
  `forget-ended-in-state` run confirms it: every fencing test passed with the ended
  leases gone from the state.
- So N24 overstated in two lines, as it stood at
  `7b6bfd1259ca3da673d17f2586671d65781627d1`: "Fencing compares lease ids against it" and
  "fencing by lease needs the ended lease's identity kept somewhere"
  (`double/SPEC-NOTES.md:220-221` and `:231-232` at that commit). What must be kept is
  the promise that ids do not repeat. Section 6 records the correction to both lines.

That promise is stated nowhere in the specification. It is the Rust's trait doc: "A
source never repeats an id over its life" (`crates/libpachinko/src/lease_ids.rs:19-22`).
The Rust checks a drawn id against the active leases only
(`crates/libpachinko/src/queue/pull.rs:100-102`), so for ended ones it trusts its source.
The double refuses any repeat, as a `Defect`, only because it still holds every lease
(`double/pachinko_double/queue.py:199-201`). So keeping ended leases does buy one thing:
it enforces the promise instead of assuming it. A counter kept beside the leases would
buy the same.

Across a restart the Rust's service starts empty (`docs/reference/http-api.md:248-249`)
and draws a new random epoch for its ids (`crates/pachinko/src/lease_ids.rs:16-19`), so
an old id landing on a new lease is improbable, not impossible.

The earliest an implementation can forget a lease is the end of the outcome that ends
it. Inside that outcome the `Freed` rules still read its pocket and consumer; the double
does (`lease_end.py:107`), and so does the specification.

**What the recommendation concedes.**

- **The letter of the specification keeps ended leases.** "An implementation detail" is
  a reading the specification does not state. Taken literally the leases are unbounded,
  one per claim, and `NeverRefusedForCapacity` names only superseded records and dead
  letters as things that may be reclaimed (1081-1085). So the Rust's empty registry is the
  implementation's choice, as the order of a lease end's entries is
  (`docs/reference/testing.md:161-170`), and the specification should say that the
  choice is allowed.
- **Nobody can be told why an action was refused.** Both implementations answer an ended
  lease and one that never existed alike. A consumer cannot tell a lapse from its own
  earlier release or from a wrong id.

**What the recommendation depends on.** Three things the specification does not settle.
Accepting the recommendation does not rule on them:

- **The lease's identity (finding 3).** "No id is issued twice" has nowhere to be said
  while the specification's lease has no id.
- **The refusal (finding 5).** If the specification named two refusals, one for an ended
  lease and one for an unknown one, an implementation would have to remember ended ids
  for as long as it promised the difference.
- **The answer to a release (finding 2).** The Rust shows the ended lease once, in the
  answer of the action that ended it: `release` returns it, and a tick's report lists the
  lapsed ones. It checks `EndedAfterGrant` there too, because "the registry keeps no
  ended lease, so the check reads the one an end returned"
  (`crates/libpachinko/tests/queue/fixture.rs:736-737`). The specification gives a
  release no answer. So in the double the snapshot is the only place a released lease's
  status and `ended_at` can be asserted, which is why the 10 ports that release read it
  there; S059, and the lapse halves of six of the 10, read it beside the tick's report.
  If a release keeps answering nothing, an implementation that forgets has nowhere to
  show `ended_at` at all.

| Option | The Rust | The double | The catalog |
| --- | --- | --- | --- |
| **A. An implementation detail: keep it or forget it (recommended)** | No change | No change. The ports and the walkthrough keep reading the ended lease | No change. The ports of S053 and S066 stand |
| B. The specification forgets an ended lease | No change | The lease is removed at the end of its outcome. At least the 11 ports and the walkthrough are rewritten. The hand-built tests that hold an ended lease go too, the `EndedAfterGrant` control among them (`test_invariants.py:154`), and the tick's report needs another source (`queue.py:314`) | No row changes |
| C. The specification keeps an ended lease and shows it on a surface | The registry keeps ended leases and needs a retention rule to stay bounded. Decision 0019 (`:80-81`), the architecture page, the HTTP reference (`http-api.md:70-73`) and `inspect.rs:82` change, with the registry assertions of S053 and S066 and the invariant walk | The lease view changes: it shows nothing without an active lease today (`consuming.py:53-57`). A decay rule if the specification adds one | No row's words change. The Rust tests behind S053 and S066 do |

**Recommendation: A.** No reader exists for an ended lease, and the specification
already shows how it speaks when it means something to be kept and read: it gives
superseded records and dead letters a TTL, a decay rule and an open question about who
reads them. B would spend a specification change on what no surface can observe, and
would take from the double the one place it can assert how a release ends a lease. C
would grow the registry for a reader nobody has asked for.

A leaves the specification's letter and its reading apart, so it comes with a proposal.
Both parts are the maintainer's to move first under `spec-change`; neither is made here:

- a sentence on `Lease` saying that no surface shows an ended lease, that no rule reads
  more of one than whether it is active, and that an implementation may forget it once
  the outcome that ends it is over;
- with finding 3, that a lease's identity is never given to another lease.

#### Question 2: is the order a pocket's records are listed in part of the contract?

**What the specification says.** A pocket's records are relationships (126-132), which
Allium makes unordered sets (`allium-skill-reference/allium/language-reference.md:612`,
`:1075`). "Queued records are ordered by position, smallest first" is a comment (162-171).
The one order a rule states is a batch's: `oldest` hands the records with the smallest
position "in that order" (467-469), and the guarantee `OldestFirst` repeats it
(1146-1150). No surface lists a pocket's records. `Consuming` lists the held records of
the caller's own lease, which is N23's question.

**What the two implementations do.** The Rust lists by key because its store is a
`BTreeMap` keyed by record key (`crates/libpachinko/src/records/memory.rs:27`), and its
docs promise it: "Its records, by key" (`inspect.rs:49`) and "Every record, by key"
(`crates/libpachinko/src/records/contract.rs:31`). The double lists records in the order
its state holds them: a new record goes last, a replaced one keeps its place, a removed
one is deleted (`double/pachinko_double/queue.py:195-220`). Neither order means anything
in the domain.

Little rests on the Rust's order. Three places tell key order from position order:
S091's last assertion (`crates/libpachinko/tests/queue/lease.rs:828-831`), one case of
the contract every Rust store must pass (`records/contract.rs:507-517`) and one doctest
(`records/contract.rs:24-28`). In S093 the two orders agree. The Rust's own model-based
test keys a pocket's records "so the store's order cannot matter"
(`crates/libpachinko/tests/queue/walk/model.rs:25`). The catalog calls S091's order "the
implementation's pinned order" (`docs/reference/testing.md:284`), and the order it means
is that of the entries a lease end creates (`:161-170`), not the listing.

Nothing rests on the double's order: the measured runs found no tagged port that reads
it, and no rule that does.

| Option | The Rust | The double | The catalog |
| --- | --- | --- | --- |
| **A. Not contract: a set, read by key (recommended)** | No change | No change | No change. The by-key ports of S091 and S093 stand |
| B. Contract, by key | No change | `Snapshot` sorts by key; the two hand-built tests change | No row changes. The ports of S091 and S093 compare the lists as the Rust does |
| C. Contract, by position | The listing changes, with S091's last assertion, the contract case, the doctest and the two docs that promise key order | `Snapshot` sorts by position; no test changes | No row changes. The ports compare the lists by position |

**Recommendation: A.** The order that matters to a consumer is position, and it already
reaches one where it matters, in a batch. A listing order would be a second statement of
it (C) or an order with no meaning in the domain (B), and the specification has no
surface to state either on. Allium also has no way to declare an ordered relationship
yet, so a stated order could only be a comment or a guarantee.

**What the recommendation concedes.**

- **The Rust does pin key order for every Rust store**, through its records contract.
  That is a promise of the Rust's `testing` API. A leaves it to the Rust to keep or drop.
- **The double's order is the only one of the three that is not a function of the
  state.** It follows a pocket's history, so two queues holding the same records can
  list them differently. That is harmless today, because the tests compare by key or
  compare two snapshots of one queue. A comparison of the double with another
  implementation by snapshot equality would need a keyed comparison or a fixed order.
  Listing by position inside `Snapshot` would give one and would change no test
  (`all-records-by-position`: none failed). It is offered as a follow-up, not done here.
- **A says nothing of the lease read's order.** That order is on a surface and on the
  wire, both implementations agree on it, and it stays `impl-choice` under finding 7.

#### Question 3: is the queued-pocket order stored and shown, or only derived?

**What the specification says.** "Queued pockets are ordered by position, smallest
first. That order is the pocket order; nothing else positions a pocket" (118-119). The
`Queue` entity stores `next_position` and nothing else (96-105). A claim takes
`min_by(free, p => p.position)` over the queued pockets (508-511). The order is total in
every state, because `UniquePocketPosition` ranges over all pockets (1023-1027). No
`exposes` block shows the order or a position.

**What the two implementations do.** The Rust keeps `pocket_order`, a set of
`(position, key)` that a claim takes its front from. Its own doc calls it "the index a
claim takes from, kept apart from `pockets` so a check can compare the two"
(`crates/libpachinko/src/queue/inspect.rs:79-81`). The check is
`order_is_the_queued_pockets` (`crates/libpachinko/tests/queue/fixture.rs:702-714`),
which S075 runs at each step. S075's own last three lines sort a sorted set and compare
it with itself (`crates/libpachinko/tests/queue/expiry.rs:825-827`), so they cannot fail.
The double keeps no index: it takes the minimum each time
(`double/pachinko_double/pull.py:142-146`).

So S075's stored-order assertion checks that an index agrees with the data it indexes.
That is a good test of an implementation that has the index, and it has no counterpart
in one that does not. What the order *is* comes from the positions in both, and the
double's port of S075 asserts exactly that: the position invariants and an empty `check`.

| Option | The Rust | The double | The catalog |
| --- | --- | --- | --- |
| **A. Only derived; a stored order is an index (recommended)** | No change | No change | No change. S075's port stands |
| B. Stored and shown | No change | A stored order is added, with a check that it matches the positions; `test_it_holds_the_whole_state` changes | No row changes. S075's port compares the stored order |

**Recommendation: A.** The specification already says it: nothing else positions a
pocket. B would add a second copy of the order to the double only so that a test could
check the copy.

**What the recommendation concedes.**

- **The vocabulary says the queue "owns the pocket order and hands out positions"**
  (57-58). "Owns" can be read as "stores". The entity's own comment is the precise one:
  what the queue owns is the counter. The wording is the maintainer's to tighten under
  `spec-change`.
- **In the Rust the index, not the positions, decides a claim.** Every invariant of the
  specification can hold while the index is wrong, so the Rust's check should stay. It is
  the implementation's check, not the specification's.
- **"Not stored" is not "not shown".** The double's tests derive the order themselves,
  with a helper called at 16 sites. A derived `pocket_order` on the double's seam would
  put that derivation where the double's own tests cover it. It is offered as a
  follow-up, not done here. Either way the order is the *queued* pockets by position: an
  idle or parked pocket keeps a stale position (N27).

#### What follows if all three are accepted

- Nothing changes in the Rust, in the double's behaviour, or in the catalog.
- The rulings on N24, N31 and N38, accepted "for now", become standing, and their notes
  say so. N24's two lines on fencing are corrected.
- Finding 1 in `double/SPEC-NOTES.md` is answered. Findings 2, 3, 5 and 7 stay open.
  Question 1 says how 2, 3 and 5 bear on it, and this section's opening covers 7.
- The proposals above go to Open points: the sentence on `Lease`, the lease's identity,
  the word "owns", and the two follow-ups for the double.
- If the double is ever used as a test oracle beside the Rust, the comparison has to
  project both snapshots first: active leases only, by id; pockets, publishers, consumers
  and each pocket's records by key; and the pocket order derived from positions. Whether the
  two shed queues need a projection too is not known (section 3, `shed-reversed`).

### 5. Review record

Two reviews ran on sections 3 and 4 before the questions were put to the maintainer.
Each was a fresh Claude agent with no part in the write-up. Their reports are
`ai_tmp/pd/PD07/review-factcheck.md` and `ai_tmp/pd/PD07/review-contract-case.md`. The
ticket names no `tickets/CONVENTIONS.md` §14 checkpoint, and none ran.

**The fact-check** read the write-up and checked each claim against its source. It
checked 62 citations and found 58 exact, reran the eight measured cases and got the same
output, and reported 22 findings. All 22 were valid and all were applied before the
ruling. The three it rated high:

| Id | Finding | Applied |
| --- | --- | --- |
| F1 | The write-up cited `forget-ended` for fencing. That case hides ended leases from `Queue.snapshot()` only, and a rule's view is built from the state, so the fencing guards still saw them | A new case, `forget-ended-in-state`, deletes them from the state. Fencing is now rested on it and on the code |
| F2 | "Only S091's last assertion and one doctest" tell key order from position order. A third does: a case of the records contract (`records/contract.rs:507-517`) | Question 2 names all three, and option C's Rust cell counts the contract case |
| F3 | "No entity refers to a lease." `Consumer.leases` and `Pocket.leases` are relationships that include ended leases | Reworded: no entity has a field that names a lease, and the relationships are read only through `active_leases` |

The other 19, by kind:

- **Claims made narrower (F4, F7, F13, F21).** The specification does read an ended lease
  after its outcome, in three places, and only for whether it is active. "The only place
  the double can assert it" is true of a release, not a lapse. The four removals are the
  only `ensures: not exists`. No `exposes` block shows a position.
- **Counts and citations (F5, F6, F10, F11, F14, F15).** `pocket_order` is called at 16
  sites, not 18. One path lacked `queue/`. `LeaseFenced` is raised at other lines than
  the range first cited. One range was a line short, one quotation sat on another line,
  and one dropped its first word.
- **Option cells (F8, F9, F19, F20).** Option B's 11 ports are a floor, since hand-built
  tests hold ended leases too. No catalog row's words change under any option; the tests
  behind the rows do. Option B of Question 3 changes the snapshot-shape test. Option C of
  Question 1 changes the double's lease view.
- **Wording (F12, F16, F17, F18, F22).** The oracle projection listed too little. S075's
  port asserts nothing about a stored order. The Rust's S010 reads no order. Finding 7 is
  covered by section 4's opening, not by Question 1. N24 overstated in two lines, not
  one.

The fact-check also ran four listings on its own. Three became cases in the runner
(`shed-reversed`, `joins-reversed` and `pockets-reversed`). The fourth, the lease view's
held records in reverse, failed S036, S085 and S090 only; it is not in `probe.txt`, so
section 3 lists it as read.

**The case for contract.** The second agent did not see sections 3 and 4. It was asked
for the strongest honest case that each shape should be part of the contract, then its
own verdict:

| Question | Its verdict | What carried the case |
| --- | --- | --- |
| 1 | Moderate that the specification's state holds ended leases and should say what becomes of them. Weak that every implementation must keep and list them | The specification removes three kinds of entity and never a lease, and filters every lease lookup on `status = active` |
| 2 | Weak | The Rust's records contract pins key order for every Rust store |
| 3 | Weak that the order is stored. Moderate that an inspection should show it, derived | Both test suites read the pocket order often, and the double's tests carry their own derivation |

Its concessions were written into section 4 before the ruling, under "What the
recommendation concedes" for each question. Two changed what was offered: Question 1's
recommended option gained the proposal that the specification say an ended lease may be
forgotten, and Questions 2 and 3 each gained a follow-up for the double. It also raised
points this ticket does not take up, which Open points carry.

The final review, `/code-review` on the diff, is recorded in section 8.

### 6. Departures and rulings

#### Maintainer's ruling on the three questions

Recorded in `ai_tmp/pd/PD07/ruling-PD07.txt`. On 2026-11-20, through AskUserQuestion, the
maintainer chose the recommended option for each question:

- **Question 1, the ended lease:** "Detail, with a spec proposal (Recommended)". An ended
  lease is an implementation detail. An implementation may keep it, as the double does,
  or forget it, as the Rust does. Nothing changes now, and the ports of S053 and S066
  stand. Open points carry the proposal.
- **Question 2, the record order:** "Not contract; read by key (Recommended)". A pocket's
  records are a set. The by-key ports of S091 and S093 stand for good. The lease read's
  order (N23) stays with finding 7.
- **Question 3, the pocket order:** "Only derived (Recommended)". A stored order is an
  implementation's index, and checking it is that implementation's test. S075's port
  stands.

The other options offered were, for Question 1, "Detail, no spec proposal", "The
specification forgets it" and "The specification keeps and shows it"; for Question 2,
"Contract, by key" and "Contract, by position"; for Question 3, "Stored and shown".

What the ruling changed in the tree, all of it words:

- `double/SPEC-NOTES.md`: a fifth row in the rulings table, finding 1 marked answered
  with what it leaves, and a Ruling line dated 2026-11-20 on each of N24, N31 and N38.
  N24's two lines on fencing are corrected in place. The count of notes is still 38.
- `double/README.md`: the paragraph that said PD07 would take the question up now states
  the ruling.
- `.scratch/python-double/README.md`: PD07's Status cell.

Nothing changed under `docs/specs/`, `crates/` or `double/pachinko_double/`, and no test
changed.

#### Other departures

- **Branch and worktree** (section 1).
- **The ticket's step 4 says "Stop" after the recommendation is put.** The maintainer
  answered in the same session, so the ruling is recorded here and the ticket is closed.
- **The Rust's `src/` was read.** PD01 to PD06's shared rule 1 keeps it unread for
  behaviour. This ticket carries no shared rules and asks for each option's consequences
  for the Rust, so `src/` was read to describe the Rust's snapshot, registry and id
  source. Nothing in the double was derived from it.
- **Files beyond the ticket.** The ticket lists no files. The two under `double/` were
  touched only to record the ruling.
- **The measurement runner is scratch.** `ai_tmp/pd/PD07/probe.py` and its output are not
  committed. Section 3 gives each case and what it wraps, so the runs can be repeated.
- **`/implement` asks for test-driven development.** There was no code to write, so it
  did not apply.
- **Two stray lines are gone.** The ticket ended with `</content>` and `</invoke>`, left
  from when it was written. They were below Open points and are removed.

### 7. Verification

Before the work commit, `just python-double` and `just check` ran on the edited tree, at
`7b6bfd1259ca3da673d17f2586671d65781627d1` with uncommitted changes. The first printed
`Ran 274 tests` and `OK`, and the second ended
`All checks passed and the worktree is unchanged.` (each exit 0;
`ai_tmp/pd/PD07/pre-commit-python-double.txt` and `pre-commit-check.txt`).

The Verification block then ran at `fe400fa34acdc319812d699aa22b3d16d988617f`, the work
commit, as one `sh` script under `set -eu`. It exited 0 (`exit=0`). The opening and
closing lines of its output:

```text
fe400fa34acdc319812d699aa22b3d16d988617f
python -B -c 'import sys; v = sys.version.split()[0]; print("python-double: Python", v); sys.exit(0 if sys.version_info >= (3, 14) else "python-double: needs Python 3.14 or newer, found " + v)'
python-double: Python 3.14.10
PACHINKO_DOUBLE_SUITE=recipe python -B -m unittest discover -s double/tests -t double -p 'test*.py'
..................................................................................................................................................................................................................................................................................
----------------------------------------------------------------------
Ran 274 tests in 0.132s

OK
…  (just check output elided; full log kept in ai_tmp/pd/PD07/verification.txt)
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
```

### 8. The final review, before the work commit

- **Harness:** Claude Code's `/code-review` at medium effort, over the uncommitted diff:
  this ticket, the index, `double/SPEC-NOTES.md` and `double/README.md`.
- **What it checked:** the hand-back's citations into the specification, the Rust, the
  handbook and the double's tests, and the counts it could reproduce (23 grep lines, 16
  `pocket_order` call sites, 38 notes, nine snapshot fields, 274 tests). They matched,
  apart from the second finding below. It did not rerun the runner's cases or
  `just check`.
- **Result:** one important finding and four nits. All five were valid and are fixed in
  the work commit.

| Finding | Severity | Disposition |
| --- | --- | --- |
| `double/README.md` said the maintainer ruled that the snapshot's listing order is the implementation's choice. The ruling covers a pocket's records only, and the order of leases reaches behaviour (N34) | important | Fixed. The paragraph names the two shapes ruled on and says the other listings are not, with the tick's tie-break as the one that reaches behaviour |
| Section 4 cited two lines of N24 that this diff rewrites | nit | Fixed. The claim is in the past tense and cites the lines at the base commit |
| The ticket was marked done with section 7 unwritten and section 8 missing | nit | Fixed by this section and by the record commit, which fills section 7 |
| N24's corrected sentence on fencing was unclear | nit | Fixed. It now says the id is looked up among all the leases and that an absent, ended or foreign one is answered alike |
| The older Ruling lines on N24, N31 and N38 still said the question stays open | nit | Fixed. Each now says it stayed open until the ruling below |

## Open points

The three questions are answered. These points are carried forward.

**Proposals for the maintainer, under `spec-change`.** None is made here. The first two
come with the ruling on Question 1.

- **Say that an ended lease may be forgotten.** A sentence on `Lease`: no surface shows
  an ended lease, no rule reads more of one than whether it is active, and an
  implementation may forget it once the outcome that ends it is over. Without it the
  specification's letter keeps every ended lease, without bound.
- **Say that a lease's identity is never given to another lease.** This needs the lease
  to have an identity first, which is finding 3 (N05). Today the promise is stated only
  in the Rust's trait doc, and the Rust checks a new id against active leases only.
- **Tighten "owns" in the vocabulary.** "Queue: ... which owns the pocket order and hands
  out positions" can be read as "stores". What the queue owns is the counter.

**Follow-ups offered for the double.** Neither is needed by the ruling.

- **List a pocket's records by position inside `Snapshot`.** The double's order is the
  only one that follows history, not state. The change breaks no test
  (`all-records-by-position`).
- **A derived `pocket_order` on the double's seam**, in place of the test helper called
  at 16 sites.

**For the Rust, outside this ticket.**

- The empty lease registry and the by-key record listing are the implementation's
  choices. `docs/reference/testing.md` names one such choice today, the order of a lease
  end's entries; these two could be named beside it.
- The last three lines of the Rust's S075 sort a sorted set and compare it with itself,
  so they cannot fail. The working check is `order_is_the_queued_pockets`.
- `check_contract`'s doc says it pins the order of `held` and `end_lease`. One of its
  cases also pins the snapshot's key order.

**Orders this ticket did not rule on.**

- Finding 7 keeps the lease view's order (N23), the order of a lease end's entries (N25,
  N29) and the order of a tick's lapses (N34). The last is a listing order that reaches
  behaviour: the double breaks a tie on `expires_at` by the order leases are listed in,
  and the Rust by lease id.
- The shed queues: the Rust lists them by expiry and arrival, the double in the order
  created. Whether the two can differ was not tested.
- Publishers and consumers: the Rust lists them by id, the double in join order. S009 and
  S010 assert the double's order.

**Findings that stay open** and bear on Question 1: 2 (what a release answers), 3 (the
lease's identity) and 5 (the refusals, and whether an ended lease and an unknown one are
told apart).
