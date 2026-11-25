---
id: T41
title: "Tick order: what one tick does first in the double and in the Rust, and what each order lets a caller or an operator see"
status: todo
depends_on: []
parallel_with: []
branch: ticket/t41-tick-order-investigation
estimated_size: M
---

# T41: Tick order: what one tick does first in the double and in the Rust, and what each order lets a caller or an operator see

## Context

`docs/specs/pachinko.allium` has three rules that wait on the clock: `LeaseLapses`
(`when: lease: Lease.expires_at <= now`), `SupersededRecordDecays` and
`DeadLetterDecays`. Each fires when its entity's `expires_at` has passed. The module
gives no order among several that are due at one `now`: not among two lapses, not among
two decays, and not between the two shed queues.

Both implementations pick an order. The Python double under `double/` recorded its
choice as note N34 of `double/SPEC-NOTES.md`, "one tick lapses in `expires_at` order,
then decays in queue order", and asked under finding 7 of that file whether the
specification should order what several lapses in one tick create, or say that the
order is not observable.

This ticket investigates and explains. It rules nothing. It changes no behaviour, no
test and no line of `docs/specs/pachinko.allium`. Its deliverable is a handbook page
for the maintainer that says what each implementation does, shows with worked examples
and diagrams what each candidate order makes visible, and ends with the questions the
maintainer must rule on and the options for each. `tickets/T42-clock-model.md`, the
clock ticket, depends on this one and lands whatever is ruled.

Nothing in this ticket re-opens a ruling. A finding that a ruling is wrong goes to the
maintainer as a question, never into the page as a recommendation to undo it.

### The rulings that bound this ticket (all 2026-11-24)

- **Finding 7, the part still open (N34): investigate first.** The order of work within
  one tick gets its own investigating ticket. The maintainer's reason: to understand the
  order fully before ruling on it. This is that ticket.
- **Finding 7, the part already ruled and applied (N23, N25, N29): by position.** The
  held records a lease view lists are smallest position first, and one lease end sends
  `Freed` smallest held position first, so the `SupersededRecord` and `DeadLetter`
  entries that one end creates are created in that order. The module states both as the
  `Consuming` guarantee `HeldRecordsOldestFirst` and in the Lease end comment. This
  orders the entries of one lease end. It says nothing about two lease ends in one
  tick, which is this ticket's subject.
- **Finding 9 (N33, N35): a tick drains.** One `now`, every due lapse, then every due
  decay, until nothing is due at that `now`; so an entry a lapse creates with a zero TTL
  decays in the same tick. The wording lands through T42. This fixes that lapses come
  before decays. This ticket investigates the order within that: among the lapses,
  among the decays, and between the two shed queues.
- **Left to T42, not to this ticket:** whether a tick is atomic as a whole; whether a
  tick with nothing due advances the latest time; what a tick reports (N33); whether the
  expiry rules' `lease.status = active` and `lease.expires_at <= now` are guards or
  restatements (N37); and the rewrite of the Lease expiry comment. Where the
  investigation meets one of these, the page says what it found and names T42. It does
  not propose an answer.

### What exists today

Re-read each at step 3; these were true when the ticket was written.

The specification:

- `LeaseLapses` requires `lease.status = active` and emits `LeaseLapsed`, which exactly
  one of `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront` and
  `LeaseExpiryIdlesEmptyPocket` answers. None of the three allots a position.
- The Lease expiry comment says "The clock emits LeaseLapsed once per active lease whose
  expires_at has passed". It names no order.
- `SupersededRecordDecays` and `DeadLetterDecays` have no `requires` and remove the
  entry.
- The `Publishing` guarantee `NeverRefusedForCapacity` orders removal for capacity by
  `shed_at` and by `nacked_at`, oldest first. It does not say how two entries with equal
  stamps are ordered, and every superseded record one tick's lapses create carries that
  tick's `now` as its `shed_at`.

The Python double:

- `Queue.tick()` in `double/pachinko_double/queue.py` reads the clock once. It applies
  each due lapse as its own outcome, in the order `due_lapses` gives. It then takes a
  fresh snapshot and applies each due superseded decay, then each due dead-letter decay,
  each as its own outcome. It commits each outcome as it goes.
- `due_lapses` in `double/pachinko_double/expiry.py` takes the active leases whose
  `expires_at <= now` and sorts them by `expires_at`. The sort is stable, so leases with
  equal `expires_at` stay in the snapshot's order, which is the order the leases were
  granted.
- `due_superseded` and `due_dead_letters` in `double/pachinko_double/decay.py` filter
  each queue and keep the queue's own order, which is insertion order. They do not sort
  by any timestamp. Say "queue (insertion) order" on the page, never "oldest first".
- `end_lease` in `double/pachinko_double/lease_end.py` sends `Freed` smallest held
  position first. So the dead letters one end creates enter their queue in held-position
  order, which can be the reverse of their `nacked_at` order.
- `TickReport` in `double/pachinko_double/queue.py` lists the lapsed leases, the decayed
  superseded records and the decayed dead letters, each as a tuple of the entities
  themselves, in the order the tick processed them.
- `double/tests/test_decay.py`,
  `test_one_tick_lapses_in_expires_at_order_then_grant_order`, pins the lapse order. It
  carries no scenario tag. The same file's
  `test_the_checker_runs_after_each_lapse_and_each_decay` shows one outcome per lapse
  and per decay.
- N34 claims that the order "changes no pocket and no record", and that it shows only in
  the order of the report and in the order in which two lapses' `Freed` rows add their
  entries to the shed queues. That is the double's own claim. Step 5 tests it; the page
  does not repeat it on trust.
- `double/README.md` draws the whole tick as a flowchart under "What stands in for time
  and for the implementation", and says a tick breaks a tie between two lapses by the
  order the leases are listed in.

The Rust, **unverified**. When the maintainer ruled, nobody had checked what the Rust
does within one tick. The list below is a first reading made while this ticket was
written. It was not run, and it is not evidence. Step 4 verifies every line against the
source and against a run, and the page cites step 4's result, never this list.

- `Queue::tick` and `tick_counting` in `crates/libpachinko/src/queue/expiry.rs` appear
  to read `now` once, lapse every lease `Leases::due` returns, then call `Shed::decay`
  once. There is one pass, not a loop.
- `Leases` in `crates/libpachinko/src/queue/leases.rs` appears to index active leases in
  a `BTreeSet<(Timestamp, LeaseId)>` named `by_expiry`, and `due` walks its prefix. If
  so, the Rust lapses earliest `expires_at` first and breaks a tie by the lease id's
  value, where the double breaks it by grant order. `LeaseId` in
  `crates/libpachinko/src/ids.rs` wraps a `u128` and derives `Ord`.
- Whether id order equals grant order then depends on the id source.
  `CountingLeaseIds` in `crates/libpachinko/src/lease_ids.rs` counts up, and
  `EpochSerialLeaseIds` in `crates/pachinko/src/lease_ids.rs` puts an epoch in the high
  64 bits and a serial in the low 64, so both would agree with grant order.
- `Shed` in `crates/libpachinko/src/queue/shed.rs` appears to keep each shed queue in a
  `BTreeMap<(Timestamp, u64), _>` keyed by `expires_at` and then an arrival counter the
  two queues share. `decay` appears to take the due prefix of the superseded queue, then
  of the dead letters. If so, the Rust's queue order is expiry then arrival, where the
  double's is insertion.
- `MemoryRecords` in `crates/libpachinko/src/records/memory.rs` keeps held records in a
  `BTreeSet<(Position, RecordKey)>`, and `held()` and `end_lease()` iterate it.
- `TickReport` in `crates/libpachinko/src/queue/expiry.rs` appears to carry
  `lapsed: Vec<Lease>`, documented "earliest `expires_at` first", and two counts,
  `decayed_superseded` and `decayed_dead_letters`. If so, the Rust's report shows no
  order among decays at all.
- The tests that appear to bear on the order:
  `crates/libpachinko/tests/queue/walk.rs` (its `tick` asserts "the lapses, in
  (expires_at, lease id) order" against the model in `walk/model.rs`);
  `crates/libpachinko/tests/queue/decay.rs`,
  `with_zero_ttls_a_lapses_entries_decay_in_the_tick_that_made_them`;
  `crates/libpachinko/tests/queue/expiry.rs`, `a_tick_examines_only_what_is_due`; and
  the unit test `due_reads_the_due_prefix_and_the_entry_that_stops_it` in `leases.rs`.
- The service appears to expose nothing of a tick to an HTTP caller. `sweep` in
  `crates/pachinko/src/actor.rs` applies `Queue::tick` and logs one `info` line with
  three counts when the report is not empty. The library's `Snapshot` in
  `crates/libpachinko/src/queue/inspect.rs`, behind the `testing` feature, lists both
  shed queues "by expiry and arrival".

The handbook and its diagrams:

- `docs/manifest.yml` registers every page under `docs/`, and
  `docs/reference/documentation-contract.md` states the rules: six manifest keys, five
  frontmatter keys equal to them, a level-one heading equal to the title, each topic in
  `canonical_for` owned by one page, at least forty words, no unfinished markers or
  placeholder prose, and every page reachable from `docs/README.md`.
- `docs/explanation/architecture.md` has a paragraph headed "The tick." on the cost of a
  tick, and `docs/reference/api.md` names `tick` and `TickReport`. Neither discusses
  order.
- `scripts/diagrams.py`, run by `just diagrams`, draws the four SVG files of decision
  0020 into `docs/decisions/0020/`. Its output directory is one constant. Each SVG
  carries light and dark colours and an opaque background, and the page that embeds it
  restates the diagram in the image's alt text. No handbook page uses a `mermaid` fence;
  `double/README.md` and `double/SPEC-NOTES.md` do.
- `tickets/T36-diagram-drift-check.md`, status `todo`, adds a check that the committed
  SVGs match the script and settles where the script lives.
- `double/` and `double/SPEC-NOTES.md` are on the branch `build-python-double`. They
  were not on `main` when this ticket was written.

### Leads the investigation must follow

These are suggestions for where to look. None is a finding, and the page states only
what step 4 and step 5 show.

1. **The tie between two lapses.** Grant order in the double, lease-id order in the
   Rust, if the first reading holds. Two leases reach an equal `expires_at` when they
   are granted, or extended by a pull, renew, ack or nack, at one `now`. Find a sequence
   of calls where grant order and the order of the last extension differ, and show what
   each implementation does.
2. **The lease id ruled under finding 3.** On 2026-11-24 the maintainer ruled that a
   lease id is a UUIDv7; `tickets/T43-caller-contract.md` works that out. A UUIDv7 is a
   millisecond timestamp followed by random bits, so two ids drawn in one millisecond
   need not order as they were drawn. If the Rust breaks a lapse tie by id, say what the
   tie-break would become under that ruling. This is a consequence to show the
   maintainer, not a reason to question the ruling.
3. **Insertion order against expiry-then-arrival.** With one TTL per queue and a `now`
   that never decreases, the two orders of a shed queue agree. Show whether they can
   differ: the double holds no guard against a clock that steps back, and the Rust's
   `Queue::now` holds the latest reading. T42 owns the clock; this ticket only shows the
   case.
4. **Where the order of two lapses' entries is read.** The double's snapshot and
   `TickReport`; the Rust's `Snapshot`; a later tick's decay order; the operator listing
   that `tickets/T39-decide-q2-q6.md` adds, whose order is one of that ticket's
   undecided points; and removal for capacity, which the module orders by `shed_at` and
   `nacked_at` without a tie-break.
5. **A tick that stops partway.** The double commits each lapse and each decay as it
   goes, so the order decides what a tick leaves behind if one outcome fails its
   invariant check. Show the case. Whether a tick is atomic is T42's question.
6. **What an HTTP caller and an operator can tell today.** If the service reports
   nothing but three counts in a log line, say so plainly: the order is then visible to
   a caller of the library and to a test, and to nobody over HTTP until T39's listing
   lands.

### Risks (`CONVENTIONS.md` §13)

- **Reading passed off as running.** A statement about what an implementation does is
  quoted from a run, or it is marked as read and not run. The Rust list above is the
  example of what must not reach the page unverified.
- **An explanation that rules.** The page lays out options and what each makes visible.
  A sentence that says which option the specification should take is a finding against
  the page.
- **A probe left in the tree.** Step 4 and step 5 may write throwaway tests to observe
  an order. None is committed. A committed test would pin an order nobody has ruled on.
- **The documentation contract.** The page ends with open questions, and the validator
  rejects unfinished markers and placeholder prose. Write each question as a complete
  sentence with its options; `just check-docs` decides.
- **Two tickets, one script.** T36 also edits `scripts/diagrams.py` and the
  `just diagrams` row of `docs/reference/commands.md`. Whichever lands second re-reads
  the other's merged text.
- **A drawing that drifts.** Until T36 lands, nothing checks an SVG against the script.
  Rerun `just diagrams` after the last edit and show that the four files of decision
  0020 did not change.
- **lychee and typos over `tickets/`.** The hand-back notes write paths and test names
  as code spans, never as links.

## Goal

The handbook has one page, `docs/explanation/tick-order.md`, from which the maintainer
can rule on the order of work within one tick without reading either implementation.
The page states what the double does and what the Rust does, each backed by a quoted
run. It shows, with worked examples and diagrams, what each candidate order makes
visible to a caller of the library, to an HTTP caller and to an operator. It ends with
the questions to rule on and the options for each. Nothing else in the repository
behaves or reads differently, and the specification is untouched.

## Non-goals

- No ruling, and no recommendation of one option over another.
- No change to `docs/specs/pachinko.allium`, to any file under `crates/`, or to the
  double's modules and tests.
- No committed test, in either implementation. Tests that pin a ruled order belong to
  the ticket that lands the ruling.
- Nothing T42 owns: tick atomicity, an idle tick and the latest time, what a tick
  reports, the N37 guards, the Lease expiry comment, the clock source.
- No re-opening of `HeldRecordsOldestFirst`, of the order within one lease end, or of
  "lapses, then decays".
- No `CHANGELOG.md` entry: its sections are per crate, and no crate changes.
- No fix for T36's gap. This ticket shows its own diagrams match the script once.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/explanation/tick-order.md` | new | The page: kind `explanation`, topic `tick_order` |
| `docs/explanation/tick-order/*.svg` | new | The page's diagrams, drawn by the script |
| `scripts/diagrams.py` | shared, own lines | The new diagrams, and an output directory per diagram where there is one constant today |
| `docs/manifest.yml` | shared, own lines | The page's entry |
| `docs/README.md` | shared, own lines | The page's link under "Understand" |
| `docs/reference/commands.md` | shared, own lines | The `just diagrams` row, which names only `docs/decisions/0020/` |
| `double/SPEC-NOTES.md`, `double/README.md` | shared, own lines, if the base holds them | One line under N34 and under finding 7 that names the page; a claim step 5 shows to be false is corrected, and the hand-back says so |
| `tickets/README.md` | shared, own lines | T41's Status cell |
| `tickets/T41-tick-order-investigation.md` | ticket | `status:` and hand-back notes |

The page lives in the handbook, not beside the double, because it compares both
implementations, the double is not on `main`, and T42 must be able to cite it from
`main`. It is an `explanation` page because it says what an order means and to whom,
not how to do a task or what an interface is.

## Steps

1. **Authorisation required: the base.** The investigation reads `double/`, which is on
   `build-python-double` and was not on `main` when this ticket was written. Check
   whether `main` holds `double/pachinko_double/queue.py`. If it does not, ask the
   maintainer whether to wait for that branch to merge or to branch from it, and stop
   until the answer. Quote the answer in the hand-back, dated.
2. **The worktree.** Create the branch and its worktree from the base step 1 settled
   (`CONVENTIONS.md` §11). Run `just initialize` (network, authorised by this step). Run
   `just check` before any edit and quote its last line as the baseline.
3. **Read.** Re-read everything "What exists today" names, in the specification, the
   double, the Rust and the handbook, and note each statement that is no longer true.
   Read N33, N34, N35 and N37 and findings 7 and 9 of `double/SPEC-NOTES.md`, and read
   `tickets/T42-clock-model.md` if the base holds it, so the page hands it what it
   needs.
4. **Verify the Rust.** Read `crates/libpachinko/src/queue/` for the tick, the lapse and
   the decay (`expiry.rs`, `leases.rs`, `shed.rs`, `settle.rs`, `pockets.rs`), the store
   in `crates/libpachinko/src/records/memory.rs`, the id sources, and the tests under
   `crates/libpachinko/tests/queue/`. Then run. For each line of the unverified list,
   and for each of these questions, record the answer with the item that shows it and a
   quoted run:
   - In what order do the leases due at one `now` lapse, and what breaks a tie?
   - In what order do the entries of two lapses in one tick sit in each shed queue?
   - In what order do due entries decay, within a queue and between the two queues, and
     can a caller tell?
   - Is one pass enough for nothing to be due when the tick returns? Say why: which
     step could make something newly due at the same `now`.
   - What does `TickReport` show, and what does the service pass on?

   An existing test that shows an answer is cited by path and name and run with
   `just test`. Where none does, write a probe test, run it with `just test`, quote its
   output in the hand-back, and remove it. Use the `testing` feature's `Snapshot` to
   read the shed queues.
5. **Verify the double, and test N34's claim.** Run `just python-double`. Confirm each
   statement under "The Python double" by the test that shows it, or by a probe run the
   same way and removed. Then test N34's claim that the order of lapses changes no
   pocket and no record: say which state a lapse writes, argue from the rules why two
   lapses in one tick cannot touch the same pocket, record, lease or counter, and back
   the argument with a probe that applies the same due set in each order and compares
   the snapshots. Do the same for the Rust as far as its public and `testing` interfaces
   allow, and say where they do not.
6. **Follow the leads.** Work through each of the six leads in Context, and anything
   steps 4 and 5 turned up. For each, either build a sequence of calls that shows a
   visible difference, with the output of both implementations quoted, or say why none
   exists.

   **Adversarial review** (`CONVENTIONS.md` §14): the record of steps 4 to 6 against the
   source, before the page builds on it. Is any statement read and not run? Does a probe
   show what it is said to show, or would it pass under another order too? Is there an
   observable the list misses?
7. **Write the page.** Use the `review-docs` skill. `docs/explanation/tick-order.md`
   holds, in this order:
   - What a tick is, in a paragraph, and which rulings already bound it. Link
     `architecture.md` for the cost of a tick; do not restate it.
   - A table of the two implementations side by side: the order of lapses, the
     tie-break, the order of each shed queue, the order of decays, the report. Each cell
     names the item that decides it.
   - The worked examples. At least: two lapses with different `expires_at`; two with an
     equal `expires_at`; two lapses that each create a superseded record and a dead
     letter; a lapse with zero TTLs whose entries decay in the same tick; and one
     example for every lead that showed a difference. Each example gives the calls, the
     state before, what each implementation does, and what a caller of the library, an
     HTTP caller and an operator can each observe.
   - The candidate orders, and for each what it makes visible and what it costs an
     implementation. Include at least: what both do today; a tie-break by grant order,
     by lease id and by pocket position; a shed queue ordered across lease ends, and
     one left unordered; and a specification that states no order and says the order is
     not observable, with what would have to be true for that sentence to hold.
   - What the page hands to T42, by name.
   - The questions for the maintainer, each with its options and what each option
     makes visible. Start from these and add or drop as the evidence requires: does the
     specification order the lapses of one tick; what breaks a tie; does it order the
     entries two lease ends create, as `HeldRecordsOldestFirst` orders one end's; does
     it order decays, or say their order is not observable.

   Register the page in `docs/manifest.yml` with frontmatter that matches, and link it
   from `docs/README.md`. Run `just check-docs`.
8. **Draw.** Add the diagrams to `scripts/diagrams.py`, written into
   `docs/explanation/tick-order/`, and run `just diagrams`. One diagram for the order of
   one tick in each implementation, and one timeline for each worked example that needs
   one. Embed each with alt text that restates it. Use no `mermaid` fence in the page.
   Show that `git diff --stat -- docs/decisions/0020` prints nothing after the run.
   Update the `just diagrams` row in `docs/reference/commands.md`. If T36 has merged,
   follow where it put the script and satisfy its check.
9. **The double's notes**, if the base holds them: the line under N34 and under finding
   7 that names the page, and any correction step 5 requires. Run `just python-double`.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Does the
   page rule or recommend? Does every statement about an implementation trace to a
   quoted run in the hand-back? Does a diagram say more, or other, than its example?
10. **Authorisation required: the page goes to the maintainer.** Present the page and
    its questions. The maintainer may rule, ask for more examples, or defer to T42.
    Quote whatever is said in the hand-back, dated. Apply no ruling here: T42 lands it.
    A request for more examples returns to step 6.
11. **Close.** Confirm no probe is left: `git status --short` lists only the files in
    Files touched. Set `status: done` here and in `tickets/README.md`, and write the
    hand-back notes. Run `just check`.

    **Authorisation required:** pushing the branch and opening the pull request are
    each asked for separately.

## Acceptance criteria

- The base was settled at step 1 before any edit, and the answer is quoted with its
  date.
- Every line of the unverified Rust list is marked confirmed or corrected in the
  hand-back, each with the item that shows it and a quoted run headed by its commit.
- The page states, for each implementation: the order of lapses and its tie-break, the
  order of each shed queue across two lease ends, the order of decays, and what the
  tick's report and the service expose.
- N34's claim that the order changes no pocket and no record is confirmed or refuted
  for each implementation, by argument and by a quoted probe.
- Each of the six leads is answered by an example on the page or by a stated reason
  that no difference exists.
- The page carries the five worked examples step 7 names, the candidate orders, the
  hand-over to T42, and the questions with their options. It recommends none.
- The page is registered, its frontmatter equals its manifest entry, it is linked from
  `docs/README.md`, and `just check-docs` passes.
- Every diagram on the page is drawn by `scripts/diagrams.py`, has alt text that
  restates it, and `just diagrams` leaves `docs/decisions/0020/` unchanged.
- The diff against the base step 1 settled names nothing under `crates/` or
  `docs/specs/`, and no file under `double/` but `SPEC-NOTES.md` and `README.md`.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed
  back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
search that must find nothing passes only on its status 1, never on an error, and
follows a positive control over the same input. Replace `main` with the base step 1
settled.

```sh
set -o pipefail
just check-docs; echo "exit $?"
just test; echo "exit $?"
just python-double; echo "exit $?"
just diagrams; echo "exit $?"
# the redraw left decision 0020's files alone: no output, and git's exit 0
git diff --stat -- docs/decisions/0020; echo "exit $?"
# the page is registered, linked and drawn (each count at least 1, exit 0)
rg -c 'explanation/tick-order\.md' docs/manifest.yml docs/README.md; echo "exit $?"
rg -c 'tick-order/[a-z-]+\.svg' docs/explanation/tick-order.md; echo "exit $?"
# positive control: the diff is read and names the page (exit 0)
git diff main...HEAD --name-only | rg -c '^docs/explanation/tick-order\.md$'; echo "exit $?"
# nothing under crates/ or docs/specs/ changed. The diff is read into a file first, so a
# failed git read cannot pass as "no match": both lines must print "exit 0"
mkdir -p ai_tmp && git diff main...HEAD --name-only >| ai_tmp/t41-names.txt; echo "git diff: exit $?"
rg -n '^(crates|docs/specs)/' ai_tmp/t41-names.txt; [ "$?" -eq 1 ]; echo "exit $?"
# the page uses no mermaid fence: rg must exit 1
rg -n '^```mermaid' docs/explanation/tick-order.md; [ "$?" -eq 1 ]; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Open points

- The base branch is step 1's to settle.
- The maintainer's answers at step 10, if any, are recorded here and land in T42.
- Whether any probe from steps 4 to 6 should become a committed test is for the ticket
  that lands the ruling. List the probes worth keeping in the hand-back, with their
  source quoted.

## Hand-back notes

None yet.

### The base (step 1)

### Baseline (step 2)

### The Rust, verified (step 4)

### The double, and N34's claim (step 5)

### The leads (step 6)

### Adversarial reviews

### The maintainer's response (step 10)

### Verification
