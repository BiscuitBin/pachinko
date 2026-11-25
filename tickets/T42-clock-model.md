---
id: T42
title: "Explore the clock model (findings 9, 10 and 13c, and finding 6's N37), have it ruled, then create the tickets that land it"
status: todo
depends_on: [T41]
parallel_with: []
branch: ticket/t42-clock-model
estimated_size: L
---

# T42: Explore the clock model (findings 9, 10 and 13c, and finding 6's N37), have it ruled, then create the tickets that land it

## Context

`docs/specs/pachinko.allium` reads `now` in most of its rules and never says what `now`
is. It names no process that notices a due lease or a due shed entry, it does not say
that time cannot run back, and it does not say whether a refused action has a time at
all. `double/SPEC-NOTES.md`, the notes of the Python double of the library, raised
these as findings 9, 10 and 13 (its time part, called 13c here) and as note N37 under
finding 6; the notes are N01, N28, N33, N35 and N37.

On 2026-11-24 the maintainer ruled that the specification is to model the clock, and
sent the design to this ticket. This ticket explores, drafts and asks. It changes no
behaviour: no rule of the module, no line under `crates/` and no line under `double/`
moves here. Its deliverables are a draft of the specification text, a decision record,
a stop for the maintainer's ruling on the points below, and then the ticket files that
update the specification, the Python double and the Rust.

Nothing in this ticket re-opens a ruling. A finding that a ruling is wrong goes to the
maintainer as a question, never into the draft as a fix.

### The rulings (all 2026-11-24)

1. **The specification models the clock** (finding 10, N28). The maintainer's reason:
   the clock is central to correct behaviour, so the specification should model it and
   so account for ordering anomalies. Time never runs back: `now` never decreases. A
   refusal does not advance it.
2. **`now` may repeat** (finding 13c, N01). Two events can share a timestamp. No rule
   depends on a resolution, and milliseconds is each implementation's choice. This
   ticket checks each rule for the equal-timestamp case.
3. **A tick drains** (finding 9, N33 and N35). One `now`; every due lapse; then every
   due decay; until nothing is due at that `now`. So an entry a lapse creates with a
   zero TTL decays in the same tick. Both implementations already do this.
4. **The clock source: anchored monotonic is the starting position.** The service reads
   the wall clock once at start, and from then on `now` is that anchor plus the
   monotonic time elapsed since: in Rust, `SystemTime` for the anchor and `Instant` for
   the elapsed time; in Python, `time.time_ns()` and `time.monotonic_ns()`. The
   specification states only that `now` never decreases, and names no source. "Hold at
   the latest", `now = max(reading, latest)`, stays as a backstop. The maintainer
   prefers a monotonic clock to relying on that hold alone.
5. **The ticket's shape.** One exploring ticket, this one: it drafts the specification,
   writes a decision record, stops for the maintainer's ruling, then creates the
   tickets that update the specification, the double and the Rust.

"Starting position" in ruling 4 means the option the draft is built on and the others
are compared with. It is not yet the final choice of source: step 2's verification can
show a reason against it, and that reason goes to the maintainer.

### What the rulings leave open

Step 6 puts each to the maintainer. None is assumed.

1. **Where "`now` never decreases" lives.** Two forms, both to be drafted:
   - (a) the specification holds the latest time as state of the `Queue` entity, with
     an invariant. This is the sketch the maintainer selected when ruling on finding
     10. The sketch fixed no wording. A suggestion of this ticket, not a ruling: each
     rule that reads `now` requires `now` to be at or after the latest time and moves
     it on, and the invariant says that no stamp of a past event (`granted_at`,
     `ended_at`, `shed_at`, `nacked_at`) is later than it. An `expires_at` is
     `now + ttl` and is later than it by design, so the invariant cannot cover every
     timestamp.
   - (b) the specification only states that `now` never decreases, as ruling 4 words
     it, and holds no such state.

   Rulings 1 and 4 each word this differently, and the maintainer left the choice to
   this ticket's draft and their own ruling on it.
2. **Is a tick atomic as a whole?** Today the Rust applies a tick in one call and the
   double commits each lapse and each decay as its own outcome, checking its invariants
   after each.
3. **Does a tick with nothing due advance the latest time?**
4. **What a tick reports** (N33), and whether the specification or a surface says so.
   The two implementations report differently today.
5. **The expiry rules' guards** (N37). `LeaseExpiryParksPocket`,
   `LeaseExpiryKeepsPocketAtFront` and `LeaseExpiryIdlesEmptyPocket` each repeat
   `requires: lease.status = active` and `requires: lease.expires_at <= now` from
   `LeaseLapses`. If the tick's definition makes noticing a lapse and applying it one
   step, the two are restatements, they are labelled as the other restating `requires`
   of the module are, and the Lease expiry comment, which gives them a job ("the lapse
   stops at its guards"), is rewritten. If noticing and applying are two steps, with
   actions able to come between, they are real guards and the comment stands.
6. **The lapse intent**, raised by the maintainer under finding 6. See its own section.
7. **The source**, if step 2 finds something against anchored monotonic, and the two
   points the source raises: whether stamps must be calendar time, and where the
   Python pair of ruling 4 lives.

### The lapse intent

The maintainer's intent, in their words as recorded: a consumer may keep renewing an
expired lease so long as no other consumer has taken the pocket; "there could
potentially be some flaws there".

That is not what is built. Today the specification, decision 0019 (its "branch A") and
both implementations end an expired lease at the next tick, whoever is or is not
waiting, so a late action succeeds only until that tick. This is a gap between the
intent and the specification, not a ruling: the maintainer has not ruled that the
behaviour changes. This ticket works the intent out far enough for the maintainer to
rule, and chooses nothing. Four points must be worked out, each with a concrete example
of states and times:

1. **Where an overdue pocket stands among the claimable pockets.** Today
   `PullClaimsFrontPocket` claims only a `queued` pocket. Under the intent a pocket
   whose lease is overdue would be claimable by another consumer while still leased.
   Is it claimed before every queued pocket, at the position it holds, or after them?
2. **When `consecutive_expiries` counts, and whether a pocket still parks.** Today the
   count rises when the tick applies the lapse, and the pocket parks at
   `config.max_consecutive_expiries`. If a lease ends only when another consumer takes
   the pocket, does the count rise then, at `expires_at`, or never, and can a pocket
   still park after N?
3. **Whether a dead consumer's records may stay held with no end** when no other
   consumer comes. Today a lapse is the only end of a dead consumer's lease, and a
   lease's end is the only time its nacked records become dead letters, its superseded
   held versions become superseded records and its deferred versions are queued.
4. **The revision of decision 0019.** Branch A and the recovery bound it states
   ("`lease_ttl`, plus up to one sweep interval, plus ...") would change.

Texts of the module the intent collides with, for the worked examples: the Config
comment's "It is a liveness timeout: a live consumer renews, and a deployment that
wants fast recovery after a consumer dies sets a short lease_ttl" (the closure of open
question 5, 2026-11-23); the Open Questions comment's closure of question 8 by decision
0019; the `Consuming` guarantee `ExpiryRecoversWithinTtl`; the invariant
`OneActiveLeasePerConsumer`; and the Lease end comment. A conflict between the intent
and a closed question is reported to the maintainer as such.

### The source options weighed

Four were weighed when ruling 4 was made. The decision record compares them again with
step 2's evidence.

| Option | What `now` is | Known cost |
| --- | --- | --- |
| Wall clock plus the hold (today) | The system time, held at the latest reading | A backward step of the wall clock stretches every active lease by the size of the step (decision 0019 says so) |
| Anchored monotonic (starting position) | A wall-clock reading taken once at start, plus monotonic time elapsed | Stamps drift from true wall time if the system clock is corrected after start. Unverified: what the monotonic clock does across a suspend |
| Pure monotonic | Monotonic time since start, with no calendar meaning | Stamps are not calendar time |
| Wall clock with refusal | The system time; a reading earlier than the latest is refused instead of held | Not defined further when it was weighed. This ticket's reading: the action or tick that read the earlier time is refused or skipped. The comparison uses that reading and labels it as this ticket's; step 6 asks the maintainer to confirm it |

Two statements about these are **unverified** and step 2 verifies them against the
standard libraries' documentation: how each monotonic clock behaves while the machine
is suspended and when the system clock is stepped, and that anchored stamps drift from
true wall time when the system clock is corrected after start. The table they came from
was written from general knowledge and is not evidence.

One statement is **a proposed requirement, not a fact**: that pure monotonic loses
calendar stamps which the operator listing of `tickets/T39-decide-q2-q6.md` wants. T39
states no requirement for calendar timestamps in its listing. Whether `shed_at`,
`nacked_at`, `granted_at`, `ended_at` and `expires_at` must be calendar time is put to
the maintainer at step 6.

### What exists today

Re-read each at step 1; these were true when the ticket was written. Line numbers move,
so each is cited by the item it names.

**The specification** (`docs/specs/pachinko.allium`).

- `entity Queue` has one field, `next_position: Integer`. It holds no time.
- Three rules fire on time: `LeaseLapses` (`when: lease: Lease.expires_at <= now`,
  `requires: lease.status = active`, `ensures: LeaseLapsed(lease: lease)`),
  `SupersededRecordDecays` and `DeadLetterDecays`. The `Consuming` surface lists
  `LeaseLapses` under `timeout:`. No surface reports a lapse or a decay.
- The three expiry rules answer `LeaseLapsed` and each repeats the two guards of open
  point 5, beside `requires: pocket.status = leased`.
- The Lease expiry comment, above `LeaseLapses`, says the clock emits `LeaseLapsed`
  once per active lease whose `expires_at` has passed, that the lapse is handled in the
  same stream as the consumer's actions, that it "stops at its guards" when an action
  processed first released the lease or moved `expires_at` on, and that a late action
  processed before the lapse succeeds. It cites decision 0019.
- `ExpiryRecoversWithinTtl`, a guarantee of `Consuming`, says a consumer whose actions
  stop loses its lease "lease_ttl after the last of them, plus the delay before the
  lapse is processed".
- Rules that stamp from `now`: `Supersede` and `DropStalePublish` (`shed_at`,
  `expires_at`), `PullClaimsFrontPocket` (`granted_at`, `expires_at`),
  `PullWithinLease`, `Renew`, the four mark rules (`lease.expires_at`),
  `NackMarksRecord` (`nacked_at`), the two Release rules and the three expiry rules
  (`ended_at`). Of the six Lease end rules, the three that create an entry stamp it
  from the `ended_at` their `Freed` carries, not from their own `now` (the Lease end
  comment says so).
- Invariants that compare timestamps: `GrantedBeforeExpiry` and `EndedAfterGrant` on
  `Lease`, the two on the shed entities that put `shed_at` and `nacked_at` at or before
  `expires_at`, and the three TTL invariants. `NeverRefusedForCapacity` orders removal
  for capacity by "oldest shed_at first" and "oldest nacked_at first".
- The module carries the comment on `entity SupersededRecord` that `shed_at` does not
  order a key's versions, and labels on the restating `requires` of
  `PullClaimsFrontPocket`, `PullWithinLease` and the two Release rules. The expiry
  rules' guards carry no such label; they wait on this ticket.

**The Rust.**

- `crates/libpachinko/src/queue.rs`: `Queue` has a field `latest: Timestamp`, and its
  private `fn now` sets `latest` to the greater of itself and the clock's reading and
  returns it. Its doc comment says the queue's `now` "never goes back, even when the
  clock does". The unit test `now_never_goes_back_when_the_clock_does` in the same file
  pins it.
- `crates/libpachinko/tests/queue/settle.rs`, the test tagged `// scenario: S085`
  (`a_changed_mark_is_refused_as_a_mark_conflict_and_changes_nothing`): it moves the
  clock to 500, has two changed marks refused, sets the clock back to 100, and asserts
  that a renew then extends from 100. Its comment reads "The queue's clock never runs
  back: had a refusal read the clock at 500, a renew at 100 would extend from 500."
  That shows a refused mark conflict does not advance `latest`. It shows nothing about
  the other refusals.
- `crates/libpachinko/src/queue/publish.rs`, `publish`: it calls `self.now()` after
  the unknown-publisher guard and before `self.pockets.publish(...)?`, the call that
  can refuse. So a publish refused for an equal rank appears to advance `latest`
  today, against ruling 1. This was read, not run; step 1 shows it with a test in
  scratch or corrects it. In `pull.rs`, `renew.rs` and `settle.rs` beside it the calls
  of `self.now()` that were read sit after the guards that refuse, but those files
  were not checked path by path. Step 1 checks every path.
- `crates/libpachinko/src/queue/expiry.rs`, `tick_counting`: it calls `self.now()`
  once and unconditionally, so a tick with nothing due advances `latest` today; it
  lapses every lease `Leases::due` returns, then calls `self.shed.decay(now)` once,
  and returns a `TickReport` of the lapsed leases and two counts. The whole tick is one
  `&mut self` call. Its helper `lapse` returns `None` when the lease is no longer
  active.
- `crates/libpachinko/tests/queue/decay.rs`,
  `with_zero_ttls_a_lapses_entries_decay_in_the_tick_that_made_them`, pins ruling 3's
  zero-TTL case.
- `crates/libpachinko/src/clock.rs` defines the `Clock` trait, one method `now`, with
  `FakeClock` beside it. `crates/pachinko/src/clock.rs` holds `SystemClock`, whose
  `now` reads `SystemTime::now()`; its doc comment says the system time can step back
  and that the queue keeps the latest reading.
- `crates/pachinko/src/actor.rs`: the ticker sends one `Tick` every `--sweep-ms`
  (default 1000, `docs/reference/configuration.md`) on its own lane, and the one owner
  task applies it.

**The Python double** (`double/`, with its notes in `double/SPEC-NOTES.md`). It lives
on the branch `build-python-double` and was not on `main` when this ticket was written.

- `double/pachinko_double/queue.py`: `Queue.tick()` reads the injected clock once, even
  when nothing is due, applies each due lapse as its own outcome, then reads the due
  decays from the state the lapses left and applies each as its own outcome, and
  answers a `TickReport` of the lapsed leases and the decayed entries themselves. For
  every other trigger the clock is read at most once per outcome and only when a rule
  reads `now`, so a refused action reads no clock.
- The double holds no latest time and no guard: nothing in `double/pachinko_double/`
  stops `now` from decreasing. `FakeClock.set` in `double/pachinko_double/effects.py`
  accepts an earlier time on purpose, and its docstring says a state reached that way
  may break an invariant such as `GrantedBeforeExpiry`.
- The double ships no real clock: `effects.py` says so, and `FakeClock` is its only
  one. Ruling 4 names a Python pair of calls, and nothing in the double would call
  them today.
- `double/pachinko_double/expiry.py`, `due_lapses`, is the filter N37 describes: only
  active leases whose `expires_at` has passed are offered.

**Decision 0019** (`docs/decisions/0019-service-runtime-tokio-and-http.md`). Three of
its paragraphs bear on this ticket:

- "Open question 8, closed with branch A": a late action the owner applies after
  `expires_at` but before the lapse succeeds; recovery after a dead consumer takes
  `lease_ttl`, plus up to one sweep interval, plus the tick and command in progress;
  "A tick samples `now` once, so a lease that expires during a long tick waits for the
  next one."
- "Clock": the real clock is `SystemTime` behind the queue's clamp; a backward step
  stretches every active lease by the size of the step; "A suspend-aware clock is a
  listed follow-up that no ticket owns yet."
- Under "What would reopen this": a machine that sleeps, or a wall clock that steps.

Scenarios S066 and S067 of `docs/reference/testing.md` are tagged `decision:0019` and
pin branch A; S006, S007, S059, S065, S073, S074 and S085 are the other rows that turn
on the clock.

### The other tickets

- **T41** (`tickets/T41-tick-order-investigation.md`) investigates the order of work
  within one tick (N34) and rules nothing. It feeds this ticket, which is why this
  ticket depends on it: the draft's tick uses the order the maintainer rules on from
  T41's handbook page, and the follow-up tickets land it; if none is ruled, the order
  stays an open point.
- **T39** (`tickets/T39-decide-q2-q6.md`) rewords the Lease expiry comment and
  `ExpiryRecoversWithinTtl` for `new_pocket_first`. This ticket's follow-up owns the
  Lease expiry comment. Whichever lands second re-reads the other's merged text and
  starts from it; each follow-up ticket written at step 7 says so.
- **T43** (`tickets/T43-caller-contract.md`) makes the lease id a UUIDv7, whose
  timestamp should come from the specified `now`. T43 notes this ticket and does not
  wait on it. T43 owns what the actions answer; what a tick reports is this ticket's.
  T43's follow-up revises decision 0019 for the lease id.
- **T45** (`tickets/T45-drop-joining.md`) revises decision 0019 for the join routes.
  With this ticket's follow-up and T43's, three tickets revise that record; whichever
  lands later re-reads the notes the earlier ones added.
- **T47** (`tickets/T47-start-rule-and-config-bounds.md`) adds a start rule that
  creates the `Queue`, and **T40** (`tickets/T40-pocket-order-model.md`) may reshape
  the `Queue` entity. Under open point 1(a) the latest time is `Queue` state, so the
  start rule gives it its first value and T40's outcome may move the field. The draft
  names both dependencies.

### Risks (`CONVENTIONS.md` §13)

- **A clock claim from memory.** No statement about `Instant`, `SystemTime`,
  `time.monotonic_ns()` or `time.time_ns()` enters the comparison or the decision
  record without a quoted source, its URL as a code span, the version it documents and
  the date read.
- **The draft is not the specification.** `docs/specs/pachinko.allium` does not change
  on this branch (`AGENTS.md`, invariant 1: the specification moves first, in the
  follow-up ticket that owns it). The draft is checked on a scratch copy of the change
  and reversed.
- **A choice slipped in as a fact.** Each open point is the maintainer's. A draft that
  only works under one answer says which answer it assumes, and gives the other.
- **Dropping the hold.** The hold is ruled to stay as a backstop. A follow-up that
  replaces `SystemClock` and removes `latest` breaks the ruling.
- **A new kind of effect.** A monotonic clock read is the clock effect behind the
  existing `Clock` trait if `pachinko` composes the anchor and the elapsed time, and
  libpachinko still performs no effect (`AGENTS.md`, invariant 2). If the draft needs a
  second trait in the library, that is a new kind of effect and needs its own decision
  record first; say so at step 6.
- **lychee and typos over `tickets/` and `docs/`.** The follow-up tickets write paths
  and URLs as code spans. The decision record is a handbook page and follows
  `review-docs`.

## Goal

The maintainer has, in one decision record, a clock model they can rule on: the draft
specification text in both forms of open point 1, a tick defined so that each of open
points 2 to 5 has its options and their consequences stated, the lapse intent worked
out on its four points, the four sources compared on verified evidence, and a table of
every rule and invariant checked for the equal-timestamp case. The maintainer's ruling
on each open point is recorded there, dated. The tickets that land the ruling exist
under `tickets/`, each self-contained, with their rows in `tickets/README.md`. Nothing
a caller can observe has changed.

## Non-goals

- No edit to `docs/specs/pachinko.allium`, to anything under `crates/`, or to anything
  under `double/`. The follow-up tickets make those.
- No ruling on the order of work within a tick: that is the maintainer's, from T41. If
  the maintainer deferred it from T41 to this ticket, step 6 asks it; this ticket still
  chooses nothing.
- No new dependency and no change to a manifest or a lockfile.
- No choice among the open points on the maintainer's behalf.
- No rewrite of decision 0019. A decision record is history; it is marked, as decision
  0020 was marked with a dated note when a ruling overtook one of its sentences.
- No design of what pull, renew, ack, nack, publish or release answer (T43).

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/decisions/<nnnn>-clock-model.md` | new | The decision record; `<nnnn>` is the next free number when step 5 runs, since T40 and T43 write records too |
| `docs/manifest.yml`, `docs/decisions/README.md`, `docs/README.md` | shared, own lines | The record's registration, its row in the index, and its link where the other decisions are listed |
| `tickets/T<nn>-*.md` | new | The follow-up tickets, numbered from the next free id when step 7 runs; each that changes the specification carries the Quint model's conditional row and step, written out in full as step 7 says, whether or not this ticket's base holds `quint/pachinko.qnt` |
| `.scratch/quint-model/QM09-clock-timestamps-decay.md`, `.scratch/quint-model/QM11-replay-lapses-and-time.md`, `.scratch/quint-model/README.md` | shared, own lines, if the base holds them, whether or not it holds `quint/pachinko.qnt` | QM09's `depends_on` and its "Blocked by" cell in the index: `T42` replaced by the id of the specification ticket step 7 creates. QM11's `depends_on` and its "Blocked by" cell: the id of the ticket that lands the ruled clock in the Rust, added if it is another ticket |
| `tickets/README.md` | shared, own lines | T42's Status cell; the follow-up tickets' rows and their place in the graph |
| `tickets/T42-clock-model.md` | ticket | `status:` and hand-back notes |

## Steps

0. **The worktree and the baseline.** Create the branch and its worktree from `main`
   after T41 has merged (`CONVENTIONS.md` §11). Run `just initialize` (network,
   authorised by this step). Run `just check` on the untouched worktree and quote its
   last line as the baseline. If `main` does not hold `double/`, read the double with
   `git show` from the branch that holds it, record that branch's commit in the
   hand-back, and edit nothing there.
1. **The re-read.** Use the `plan-change` skill. Re-read everything "What exists today"
   names and record each sentence that no longer matches; work from what the tree
   says. Read T41's merged handbook page and any ruling recorded from it. Then, for
   the Rust, list every path that returns a refusal from `publish`, `pull`, `renew`, `release`,
   `ack` and `nack`, and show for each whether `self.now()` runs before it; for the
   double, do the same for its refusals. The result is a table in the hand-back. A
   refusal that reads the clock is a finding for the maintainer, not a fix.
2. **The clocks, verified.** **Network, authorised by this step**, read-only. Read and
   quote, each with its URL as a code span and the date read:
   - Rust, at the release `rust-toolchain.toml` pins (1.99.1 when this ticket was
     written): the documentation of `std::time::Instant` and `std::time::SystemTime`,
     `https://doc.rust-lang.org/<version>/std/time/struct.Instant.html` and
     `.../struct.SystemTime.html`, including the "Underlying System calls" and
     monotonicity sections, for what `Instant` does while the machine is suspended on
     each platform the service runs on, and what a step of the system clock does to
     each type.
   - Python, at the version `pyproject.toml` pins (3.14 when written): the `time`
     module, `https://docs.python.org/<version>/library/time.html`, for
     `time.monotonic_ns()`, `time.time_ns()` and `time.get_clock_info()`, and PEP 418,
     `https://peps.python.org/pep-0418/`, for suspend and for clock steps.
   - tokio, at the version `Cargo.lock` pins: what clock `tokio::time::sleep` runs on,
     since the ticker's sweep is one, from the pinned source in the registry cache or
     its documentation.

   Fill the table of the four source options from those quotes alone. Where the
   documentation is silent or says the behaviour is platform-specific, write that; do
   not fill the cell from general knowledge. Show by a worked example that anchored
   stamps drift from true wall time when the system clock is corrected after start,
   or correct the claim.
3. **The equal-timestamp check.** One row for each rule and each invariant of the
   module that reads `now` or compares two timestamps, and for
   `NeverRefusedForCapacity`'s two orders: what the rule or invariant does when two
   events share a timestamp, and whether that is defined, an order left open, or a
   defect. At least these cases: a lease granted and due at one `now`
   (`lease_ttl = 0`, S006); an action and a lapse at one `now`; two leases with equal
   `expires_at` in one tick (T41's subject); two shed entries with equal `shed_at` or
   equal `nacked_at` under the capacity order; `ended_at = granted_at`; a nack and its
   lease's end at one `now`. Also state whether any rule needs a resolution, with the
   evidence. A defect is reported at step 6 and fixed nowhere in this ticket.
4. **The draft.** Use the `spec-change` skill for the Allium, and read only the
   reference pages it points to. Write the draft as a patch against
   `docs/specs/pachinko.allium`, kept under `ai_tmp/t42/`, in two variants for open
   point 1, form (a) and form (b). Each variant gives: the statement that `now` never
   decreases and that a refusal does not advance it; the tick, one `now`, the lapses,
   then the decays, until nothing is due, with a sentence on why one pass of each
   reaches that state or why it does not; the rewritten Lease expiry comment and
   `ExpiryRecoversWithinTtl`; and, for open points 2 to 5, the text under each answer.
   For open point 5 give both readings in full: the labels on the two guards and the
   rewritten comment, and the comment as it stands with the reason the guards are
   real. For the lapse intent, write the worked examples of its four points, and
   sketch the rules that would change; do not draft them as the specification's text
   unless the maintainer rules the behaviour changes.

   Check each variant's syntax without moving the specification: apply the patch with
   `git apply`, run `just check-specs` and `just analyse-specs` and quote their output
   and exit status, then reverse the same patch with `git apply -R` and show
   `git status --short` empty of the module. If a variant cannot be made to pass, say
   so and quote the first failure; an unchecked draft is labelled unchecked.
5. **The decision record.** Use the `review-docs` skill. Write
   `docs/decisions/<nnnn>-clock-model.md` in the shape of decision 0020 (Context,
   Decision, Consequences, What would reopen this, Related pages), with its Decision
   section marked as proposed and awaiting the maintainer's ruling. It holds: the five
   rulings with the maintainer's reason; the source comparison with step 2's quotes;
   step 3's table; both variants of the draft, as fenced text; each open point with its
   options and what each costs; the lapse intent's four points; and which paragraphs
   of decision 0019 a ruling would overtake (branch A, "Clock", and the reopen
   bullet). Register it in `docs/manifest.yml`, `docs/decisions/README.md` and
   `docs/README.md`. Run `just check-docs`.

   **Adversarial review** (`CONVENTIONS.md` §14), over the decision record, the two
   patches and the hand-back tables, before the maintainer is asked. Ask: whether any
   statement about a clock lacks a quoted source and version; whether the record
   states a ruling the maintainer did not make, or presents the lapse intent or the
   calendar-stamp point as decided; whether either variant changes behaviour beyond
   the rulings; whether the draft tick can leave something due at its `now`; whether
   step 3 missed a rule or invariant that reads a timestamp; whether the hold is still
   present in every option built on anchored monotonic; whether a refusal can advance
   the latest time under form (a); and whether anything outside Files touched changed.
6. **Authorisation required: the maintainer's ruling.** Give the maintainer the
   decision record and ask, quoting this ticket's Context, in this order:
   - open point 1, form (a) or form (b);
   - open points 2, 3 and 4 on the tick: atomic or not, whether an idle tick advances
     the latest time, and what a tick reports and where that is stated;
   - the order of work within a tick, if T41 left it unruled or the maintainer
     deferred it to this ticket, with T41's page and its options;
   - open point 5, the N37 guards, with what the answer to the tick implies for it;
   - the lapse intent: whether the behaviour changes, and if so each of its four
     points;
   - the source: whether anchored monotonic stands on step 2's evidence; whether
     stamps must be calendar time; whether the double gains a real clock or the Python
     pair is recorded as guidance only; the reading of "wall clock with refusal";
   - each defect or clock-reading refusal steps 1 and 3 found, the refused equal-rank
     publish among them if step 1 confirms it;
   - whether decision 0019 is marked by the follow-up ticket that changes the code, as
     this ticket proposes, so that the record stays true of the code until then, or
     here;
   - the follow-up tickets' split, with step 7's proposal, and their base branch,
     given where `double/` lives.

   Then stop and do nothing further until the maintainer answers. Quote the answers in
   the hand-back, dated. An answer that names something outside these points is a new
   design: record it and stop again.
7. **The ruling recorded, and the follow-up tickets.** Set the decision record's
   Decision section to what was ruled, with the date, keeping the options that lost
   and why. Then write the follow-up tickets, each to `CONVENTIONS.md` §11 and §14 and
   in the shape of this one, self-contained, with the ruling restated in it. The split
   is the one the maintainer gave at step 6. The proposal put to them there:
   - one ticket for the clock model: the specification first within it (`AGENTS.md`,
     invariant 1), with the Lease expiry comment, the N37 guards or their labels and
     `ExpiryRecoversWithinTtl`; then the Rust library and service and the Python
     double, `double/SPEC-NOTES.md` (findings 9, 10 and 13, and N01, N28, N33, N35 and
     N37 marked as applied), the scenario catalog, and the dated note in decision
     0019. The maintainer has so far landed a ruling that adds behaviour with its code
     and tests in one ticket, so that the specification and the code agree at every
     commit; a specification-only ticket is offered as the alternative, not assumed;
   - one more for the lapse intent, if the maintainer ruled that the behaviour
     changes, since it moves rules, scenarios S066 and S067 and decision 0019's branch
     A.

   Each names its dependencies on T39, T40, T43, T45 and T47 as "The other tickets"
   states them. Add their rows and their place in the graph to `tickets/README.md`.

   **The Quint model.** A Quint model derived from the specification,
   `quint/pachinko.qnt`, may exist when a follow-up runs, whether or not this ticket's
   base holds it. So each follow-up ticket that changes `docs/specs/pachinko.allium`
   carries the model's obligation, written out in full and conditional on the tree it
   lands on, at its base or after its closing merge or rebase, which its closing step
   reads again, doing the model's step again there if the merge brought in or
   changed the model, a mutant patch, the fixtures or a replay: a follow-up never
   cites a scratch path or this ticket for it. Its Files
   touched has one row, classed
   `shared, own lines, if the tree this ticket lands on holds quint/pachinko.qnt`, for
   `quint/pachinko.qnt`, `quint/mutants/*.patch`, `quint/traces/*`,
   `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`,
   `crates/libpachinko/tests/queue/walk/model/trace.rs` and
   `double/tests/test_replay.py`, each only where that tree holds it. Its steps have
   one, under the same condition, that says all of this in its own words: the model
   moves in the same change wherever it models what the ticket changes (the same-change
   rule in `AGENTS.md`, and the decision record under `docs/decisions/` that introduced
   the model); each mutant patch the edit breaks is re-cut, keeping its mutation and the
   property it names, and listed in the hand-back; if `quint/traces/` exists,
   `just quint-traces` regenerates it, and each replay that exists stays green, the
   Rust test in `just test` and `double/tests/test_replay.py` in `just python-double`,
   with a field or a step the model gained either mapped or added to that replay's own
   list of what it does not compare, beside the ticket that will map it, and a step
   only with the maintainer's answer on which ticket that is; then
   `just check-quint` runs, and
   `just verify-quint` if the recipe exists, the fixtures first because both gates fail
   on a model whose hash the fixtures do not record. The step names what the ticket
   touches in the model: `LeaseLapses`, the lease extensions, the two decay rules and
   `ExpiryRecoversWithinTtl`. While the model has no clock (no `now` in
   `quint/pachinko.qnt`), a ticket that only says when something is due changes nothing
   in the model and says so in its hand-back; the Quint series' own clock ticket adds
   the clock afterwards.

   **The Quint series' two blockers**, if the base holds the files named here, whether
   or not it holds `quint/pachinko.qnt`. The series' clock ticket, QM09, waits for the
   follow-up that changes the specification: replace `T42` with that ticket's id in
   `depends_on` of `.scratch/quint-model/QM09-clock-timestamps-decay.md` and in QM09's
   "Blocked by" cell of the index in `.scratch/quint-model/README.md`. The series'
   ticket QM11 replays the model's timed traces in the Rust, and waits for the follow-up
   that lands the ruled clock in the Rust: if that is another ticket than the one
   above, add its id to `depends_on` of
   `.scratch/quint-model/QM11-replay-lapses-and-time.md` and to QM11's "Blocked by" cell
   of the same index. If the base does not hold those files, name both ids in the
   hand-back as owed to the series.

   **Adversarial review** (`CONVENTIONS.md` §14), over the recorded ruling and the new
   ticket files. Ask: whether each ticket can be run by an agent with no context;
   whether any restates a ruling differently from the maintainer's quoted answer;
   whether the specification moves first; whether a fact is cited by a line number
   that will move; whether the hold survives; and whether two tickets own the same
   lines.
8. **Merge and gate.** Use the `project-check` skill: `just check`, with `fix-quality`
   for any failing recipe. Run `git fetch origin` (network, authorised by this step)
   and merge `origin/main`. Rerun the Verification block at the merge commit and
   requote it. Set `status: done` here and T42's Status cell in `tickets/README.md`,
   and commit with the hand-back notes, each quoted output headed by the commit it ran
   at.

   **Authorisation required:** pushing the branch and opening the pull request are
   each asked for separately.

## Acceptance criteria

- The baseline `just check` line is quoted, and step 1's record says which sentences
  of "What exists today" held.
- The hand-back holds a table of every refusal path in the Rust and in the double,
  each marked as reading the clock or not, with the file and item that shows it.
- Every cell of the source comparison rests on a quoted source with its URL, version
  and date read, or says the source is silent. No cell is filled from memory.
- Step 3's table has a row for every rule and invariant of the module that reads `now`
  or compares timestamps, and each case the step lists.
- Both variants of the draft exist, each with `just check-specs` and
  `just analyse-specs` output quoted, or labelled unchecked with the first failure.
- The decision record is registered, passes `just check-docs`, and holds the rulings,
  the comparison, the table, both drafts, the open points and the lapse intent's four
  points.
- The maintainer's answers at step 6 are quoted in the hand-back with their date, and
  nothing before them chose an option.
- The decision record's Decision section states what was ruled, and the follow-up
  tickets exist, split as the maintainer ruled, each with its row in
  `tickets/README.md`; in each, the specification moves before the code.
- `git diff --name-only origin/main...HEAD` lists only paths under `docs/decisions/`,
  `docs/manifest.yml`, `docs/README.md` and `tickets/`.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed
  back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
line that must print nothing follows its positive control.

```sh
set -o pipefail
# the evidence Context cites is where it says (each must match, exit 0)
rg -n 'self\.latest = self\.latest\.max\(self\.clock\.now\(\)\)' crates/libpachinko/src/queue.rs; echo "exit $?"
rg -n "The queue's clock never runs back" crates/libpachinko/tests/queue/settle.rs; echo "exit $?"
rg -n 'SystemTime::now\(\)' crates/pachinko/src/clock.rs; echo "exit $?"
rg -n '^rule LeaseLapses|^-- Lease expiry -|@guarantee ExpiryRecoversWithinTtl' docs/specs/pachinko.allium; echo "exit $? (must print 3 lines)"
rg -n 'closed with branch A' docs/decisions/0019-service-runtime-tokio-and-http.md; echo "exit $?"
# the record is registered
rg -c 'clock-model' docs/manifest.yml docs/decisions/README.md docs/README.md; echo "exit $? (must print three count lines, one per file)"
just check-docs; echo "exit $?"
# scratch work is ignored
git check-ignore -v ai_tmp/t42; echo "exit $? (must be 0)"
# positive control: the branch changed its own files
git diff --name-only origin/main...HEAD; echo "exit $? (docs/decisions, docs/manifest.yml, docs/README.md and tickets/ only)"
# then nothing that carries behaviour (must print nothing)
git diff --stat origin/main...HEAD -- docs/specs crates double Cargo.toml Cargo.lock Justfile; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Open points

- Open points 1 to 7 of Context, the lapse intent's four points and the reading of
  "wall clock with refusal" are step 6's to settle.
- Whether timestamps must be calendar time is a proposed requirement, not a fact.
- How each monotonic clock behaves under suspend and under a clock step is unverified
  until step 2 runs.
- Whether every refusal path in the Rust avoids reading the clock is unverified until
  step 1 runs; only the mark conflict is pinned by a test (S085), and a refused
  equal-rank publish appears to read it.
- The order of work within a tick is T41's and the maintainer's.
- The follow-up tickets' ids and the decision record's number are the next free ones
  when their steps run.
- Whether the follow-up tickets can start from `main` depends on where `double/` is
  when they are written.

## Hand-back notes

### Baseline

### The re-read and the refusal paths (step 1)

### The clocks, verified (step 2)

### The equal-timestamp check (step 3)

### The draft and its checks (step 4)

### The decision record and review T42-1 (step 5)

### The maintainer's ruling (step 6)

### The follow-up tickets and review T42-2 (step 7)

### Merge and gate (step 8)
