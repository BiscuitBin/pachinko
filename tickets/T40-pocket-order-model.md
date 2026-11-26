---
id: T40
title: "Decide where the pocket order lives: a position on the Pocket, or an order the Queue owns"
status: todo
depends_on: [T35]
parallel_with: []
branch: ticket/t40-pocket-order-model
estimated_size: M
---

# T40: Decide where the pocket order lives: a position on the Pocket, or an order the Queue owns

## Context

`docs/specs/pachinko.allium` gives every `Pocket` a `position: Integer`. The queued
pockets are ordered by it, smallest first, and that order is the pocket order. A pocket
that is idle or parked is outside the pocket order and still carries the position it
last had.

A Python double of the library was built from the specification on the branch
`build-python-double`, and its notes, `double/SPEC-NOTES.md`, list 15 findings, each a
question for the maintainer. Finding 12 (note N27, scenario S055) asked: "Does an idle
pocket's position mean anything?" An idle pocket keeps the position it had, which no
rule reads and a revival replaces.

This ticket explores the question the maintainer asked in reply, puts a recommendation
to the maintainer, records the ruling, and writes the tickets that carry the ruling out.
It changes no behaviour: not the specification, not the Rust, not the service, not the
double.

### The ruling on finding 12 (2026-11-24)

**Not ruled as asked.** The maintainer did not answer "does an idle pocket's position
mean anything?" and asked a wider question instead. As recorded on 2026-11-24, not a verbatim quote:

> Should `position` be a property of the Pocket at all, or a concern of the Queue the
> pockets sit in?

What is decided, and this ticket never re-opens it:

- The wider question is explored by this ticket before anything changes. The
  specification is unchanged for finding 12 until the maintainer rules on this ticket's
  recommendation.
- **Pockets only.** The question is about the pocket order. A record's `position`, its
  `deferred_position` and the order of a pocket's records are out of scope.
- The ticket compares the models, drafts the specification for the one it prefers,
  writes a decision record, stops for the maintainer's ruling, and after the ruling
  creates the tickets that perform the updates: the specification, the Rust, the
  service, the Python double and the scenario catalog. It changes no behaviour itself.
- **The leaning, if position stays on the Pocket.** Before widening the question the
  maintainer selected the option "clear it on idling": `position` becomes optional and
  is null while the pocket is idle. This is recorded as the leaning for that branch of
  the decision. It is not a ruling on the wider question, and it says nothing about a
  parked pocket (open point 2).

No reason of the maintainer's for the wider question was recorded with it. A finding
that the ruling above is wrong goes to the maintainer as a question, never into a fix.

### What exists today

Re-read each at step 2; these were true when the ticket was written, on the branch
`build-python-double`. Rules, invariants and items are cited by name, not by line: other
tickets move the lines.

**The specification, `docs/specs/pachinko.allium`.**

- `entity Pocket` declares `position: Integer`, with no `when` clause, so every pocket
  has one in every state. The comment above the entity says: "Queued pockets are ordered
  by position, smallest first. That order is the pocket order; nothing else positions a
  pocket."
- `entity Queue` has one field, `next_position: Integer`. Its comment says every rule
  that allots a position takes `next_position` and advances it, "so positions are unique
  and the order of allotment is the order of the queue". **Pockets and records draw from
  the same counter.**
- The vocabulary comment at the head of the module says: "Queue: the deployment's one
  queue, which owns the pocket order and hands out positions", and "Position: where a
  pocket stands in the pocket order and a record in its pocket: the queue's counter when
  it entered or re-entered, smallest first. ... a release or revival gives a pocket a
  fresh one."
- **One rule reads a pocket's position.** `PullClaimsFrontPocket` takes
  `let free = Pockets where status = queued` and
  `let pocket = min_by(free, p => p.position)`. `min_by` is a black box, described in
  the comment above the Pull rules.
- **Four rules write it**, each from `queue.next_position`, which each then advances:
  `PublishIntoNewPocket` (the pocket takes `next_position` and its first record the
  value after, so the rule advances the counter by two), `ReviveIdlePocket`,
  `ReviveParkedPocket` and `ReleaseRequeuesPocket`.
- **A lapse keeps it.** `LeaseExpiryKeepsPocketAtFront` sets `pocket.status = queued`
  and does not mention `position`; the Lease expiry comment says "position is untouched
  on the queued path, so the pocket keeps its place at the front." So a leased pocket's
  position is not stale: it is the place the pocket returns to if its lease lapses.
- **Idle and parked pockets carry a stale one.** `ReleaseIdlesEmptyPocket`,
  `LeaseExpiryIdlesEmptyPocket` and `LeaseExpiryParksPocket` change `status` and leave
  `position` alone. No rule reads the position of an idle or parked pocket, and the two
  Revive rules replace it.
- **Two invariants range over every pocket, in any state:** `UniquePocketPosition`
  (`a != b implies a.position != b.position`, for `a` and `b` in `Pockets`) and
  `PocketPositionsBelowNext` (`p.position < queue.next_position`). The stale positions
  satisfy both, because they came from the same counter.
- On 2026-11-24, under finding 6, a comment was added inside `PullClaimsFrontPocket`
  above `requires: pocket.status = queued`: "Restates the selection: pocket is one of
  the queued pockets, so this cannot fail." The selection it restates is the `min_by`
  over `position` on the line above it.
- The record side already has a field that exists in one state only:
  `deferred_position: Integer? when status = held` on `entity Record`.

**What a lapse does to the order: the case every model must reproduce.** Pocket A is
created at position 0 (its record at 1) and pocket B at position 2 (its record at 3).
Consumer c1 claims A and consumer c2 claims B. Both leases lapse, in either order, below
the parking threshold. Each pocket is queued again at the position it was claimed from,
so the pocket order is A then B whichever lease lapsed first, and both stand ahead of a
pocket C that was queued at position 4 and never claimed. A model in which a lapsed
pocket "goes to the front" of a list gives B then A when A lapses first. Derived from
the rules above when the ticket was written; step 2 confirms it by reading the three
expiry rules, and step 3 runs every model through it.

**The Rust library.**

- `crates/libpachinko/src/queue/pockets.rs` keeps both forms. `struct Pocket<R>` has a
  field `position: Position`. `struct Pockets<R>` holds `by_key: BTreeMap<PocketKey,
  Pocket<R>>`, `order: BTreeSet<(Position, PocketKey)>` and `next_position: Position`.
  `order` holds the queued pockets only.
- `front` reads `order.first()` and `claim_front` takes `order.pop_first()`, so a leased
  pocket is out of `order` and keeps its `position` field. `create`, `touch` (the two
  revivals) and `release` take a position with `take_position` and insert into `order`.
  `lapse` re-inserts `(pocket.position, key)` on the queued path, with the position the
  pocket was claimed from, and inserts nothing when the pocket idles or parks.
- `Position` is a `u64` newtype in `crates/libpachinko/src/records.rs`, shared by
  pockets and records.
- The inspection seam, `crates/libpachinko/src/queue/inspect.rs`, exported by the
  `testing` module, shows both: `PocketView.position` ("Its position in the pocket
  order, or where it last stood there") and `Snapshot.pocket_order` ("The pocket order
  as the queue holds it, front first: the index a claim takes from, kept apart from
  `pockets` so a check can compare the two"), beside `Snapshot.next_position`. Its
  doctests assert absolute values (a first pocket at `Position::new(0)`, its record at
  `Position::new(1)`).
- `crates/libpachinko/tests/queue/fixture.rs` checks `unique_pocket_position`,
  `pocket_positions_below_next` and `order_is_the_queued_pockets` (the index equals the
  queued pockets sorted by position). The walk model,
  `crates/libpachinko/tests/queue/walk/model.rs`, keeps its own `pocket_order` and
  chooses the front pocket by `min_by_key` over the pockets' positions, "not from the
  queue's order index".

**The service.** A search for `position` over `crates/pachinko/src` and
`crates/pachinko/tests` found nothing when the ticket was written: no route, body or
flag shows a pocket's position. Step 2 repeats the search.

**The Python double** (`double/pachinko_double/`, on the branch `build-python-double`
only).

- It keeps no order index. `model.py` gives `Pocket` a `position: int`; `queue.py`
  holds the pockets in a dict by key, with `next_position` starting at 0; `pull.py`
  derives the front pocket at each claim as the `min` over the queued pockets by
  `position`.
- `publish.py` and `settle.py` write the position in the four rules above through
  `allot_position`; `expiry.py` and `settle.py` leave it alone on a lapse and on idling,
  each with a comment citing N27.
- `invariants.py` checks `UniquePocketPosition` and `PocketPositionsBelowNext`;
  `inspection.py` exposes `next_position`; scenario S055's test is in
  `double/tests/test_settle.py`; `double/README.md` draws the pocket lifecycle with its
  positions and prints `position=` in its worked transcripts.

**The handbook.** `docs/project/terminology.md` carries the Pocket, Queue, Position and
"Pocket order" entries in the specification's words. `docs/reference/testing.md` is the
scenario catalog; S022 ("does not move it in the pocket order") and S055 ("A release
idles an empty pocket") are the rows closest to this question.
`docs/explanation/architecture.md` describes the snapshot's pocket order.

**Allium, unverified until step 2.** `allium-skill-reference/allium/language-reference.md`
said, when the ticket was written, that `List<T>` is an ordered field type, that
`Sequence<T>` is what an ordered relationship produces, that `.first` and `.last` are
for ordered collections only, and that "the declaration syntax for ordered relationships
... is pending a follow-up ALP". Whether Allium 3, as the pinned checker implements it,
can state an order the Queue owns (insert at the back, remove, keep a place) has not
been tried. Step 2 reads the reference and step 4 tries the draft against the pinned
checker. Nothing in this ticket assumes what a list can express.

### Candidate models

Suggestions to start the comparison from, not rulings. The executor may add or split
models; each one compared is named and defined in the decision record.

- **A. As today.** `position: Integer` on every pocket in every state; idle and parked
  pockets carry a stale value.
- **B. On the Pocket, present only while it means something.** The maintainer's leaning
  for this branch: optional, and null while idle. Sub-choices to lay out: parked too
  (open point 2); a leased pocket keeps it, since a lapse returns the pocket to it;
  the form (`Integer?` set and cleared by rules, or a `when status = ...` clause as
  `Record.deferred_position` has); and the null guards `UniquePocketPosition` and
  `PocketPositionsBelowNext` then need.
- **C. On the Queue, as an ordered collection of pockets.** The Pocket has no position.
  Sub-choices: whether the collection holds queued pockets only, or queued and leased
  ones so that a lapsed pocket is still in its place; what replaces the two invariants;
  what `PullClaimsFrontPocket` reads.
- **D. On the Queue, as entries.** An entity the Queue owns pairs a pocket with a
  position while the pocket is in the order, as the Rust's
  `order: BTreeSet<(Position, PocketKey)>` does; the Pocket has no position field.

### Other tickets this one touches

Tickets T40 to T47 were written together on 2026-11-24. Re-read what each has merged at
step 2.

- **T39** (`tickets/T39-decide-q2-q6.md`, `new_pocket_first`) depends on this ticket, so
  that its model is designed once, on the pocket-order model that wins. Its suggested
  model, a Boolean on `Pocket` that `PullClaimsFrontPocket` prefers, with "positions
  stay one rising counter", assumes pocket positions as they are today. This ticket's
  outcome may remove the pocket positions that text mentions.
- **T47** (`tickets/T47-start-rule-and-config-bounds.md`, the start rule) settles the
  Queue's first position (finding 13; notes N04 and N16) and waits on this ticket for
  it. The counter is shared: a model in which pockets take no value from
  `next_position` shifts every record's absolute position (a first record would stand
  at the first position, not one after it), which moves the six scenario rows N04 and
  N16 name (S011, S012, S014, S015, S016, S022) and the inspection doctests. The
  comparison states, for each model, what the counter still allots.
- **The finding 6 comment** in `PullClaimsFrontPocket` ("Restates the selection") may
  need rewording if the selection is no longer by position. T46
  (`tickets/T46-no-claim-without-records.md`) also edits that rule and that comment.
- **T42** (`tickets/T42-clock-model.md`) owns the Lease expiry comment, whose last
  sentence is about position, and may change what a lapse does. If T42 has merged a
  change to where a lapsed pocket stands, the comparison starts from it.
- **T42 and T43** also write decision records and follow-up tickets. Decision numbers
  and ticket ids are taken at the time of writing, never from this text.

### Risks (`CONVENTIONS.md` §13)

- **A model that quietly changes the order.** The pocket order is observable: it decides
  which pocket a pull claims. Each model is run through the worked cases of step 3. A
  model that gives a different order in any of them is not rejected by the executor; the
  difference is put to the maintainer as an explicit point.
- **Scope creep into records.** The counter and the `Position` vocabulary entry are
  shared with records. The comparison says what each model does to them; it proposes no
  change to how records are ordered.
- **A draft that is never checked.** A draft specification the pinned checker has not
  read may not be Allium. Step 4 checks it and quotes the output.
- **The specification must not change in this ticket.** Step 4 applies the draft to the
  module in the working tree to check it, then removes it. The Verification block shows
  `docs/specs`, `crates` and `double` unchanged against the base.
- **The specification moves first** in the follow-up tickets (`AGENTS.md`, invariant
  1), and the Python double and `double/SPEC-NOTES.md` move with the Rust.
- **A decision nobody can find.** The record is registered in `docs/manifest.yml` and
  `docs/decisions/README.md` in the commit that adds it.
- **lychee and typos over `tickets/`.** The hand-back notes and the new tickets write
  paths and names as code spans, never as links.

## Goal

The maintainer has ruled where the pocket order lives, on a written comparison of the
models, a draft specification for the recommended one that the pinned checker accepts,
and a decision record. The record carries the ruling and its date. A ticket exists for
each update the ruling needs, and `tickets/README.md` lists them. The specification, the
crates and the double are byte-for-byte what they were.

## Non-goals

- No change to `docs/specs/pachinko.allium`, `crates/` or `double/`, including
  `double/SPEC-NOTES.md`. The follow-up tickets make them.
- No change to a record's `position`, to `deferred_position`, or to how a pocket's
  records are ordered.
- No change to what a caller can observe. If the recommended model changes the order in
  any case, that is the maintainer's decision at step 6 and a follow-up ticket's work.
- No design of `new_pocket_first` (T39), of the Queue's first position (T47) or of what
  a lapse does (T42). This ticket says what its outcome means for each.
- No answer assumed for any open point.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/decisions/<nnnn>-pocket-order.md` | new | The decision record: the question, the models compared, the worked cases, the draft specification for the recommended model, the consequences, and after step 6 the ruling with its date. `<nnnn>` is the next free number at step 5 (0021 when the ticket was written) |
| `docs/decisions/README.md` | shared, own lines | The record's row, and the "next decision is" sentence |
| `docs/manifest.yml` | shared, own lines | The record's entry, shaped as decision 0020's |
| `docs/README.md` | shared, own lines | Only if `just check-docs` requires the record to be linked there; decision 0020 is reached through `docs/decisions/README.md` alone |
| `tickets/T<nn>-<slug>.md`, one per follow-up | new | The update tickets step 7 writes, at the next free ids; each that changes the specification carries the Quint model's conditional row and step, written out in full as step 7 says, whether or not this ticket's base holds `quint/pachinko.qnt` |
| `tickets/README.md` | shared, own lines | T40's Status cell; the follow-up tickets' rows and their place in the graph |
| `tickets/T39-decide-q2-q6.md`, `tickets/T47-start-rule-and-config-bounds.md` | shared, own lines | Only as step 6 authorises: `depends_on`, and a dated note on what the ruling means for each |
| `tickets/T40-pocket-order-model.md` | ticket | `status:` and hand-back notes |

## Steps

0. **The base, the worktree and the baseline.**

   **Authorisation required: the base.** `double/`, `double/SPEC-NOTES.md` (finding 12,
   N27) and the finding 6 comment were on the branch `build-python-double` and not on
   `main` when this ticket was written. Before creating anything, check whether `main`
   holds them (`git ls-tree --name-only origin/main double/`). If it does not, say so
   and ask the maintainer whether to wait for that branch to merge or to work from it.
   Do not choose.

   Then create the branch and its worktree from the base, after T35 has merged
   (`CONVENTIONS.md` §11). Run `just initialize` (network, authorised by this step). Run
   `just check` on the untouched worktree and quote its last line as the baseline.
1. **Read.** Use the `plan-change` skill. Read the whole of
   `docs/specs/pachinko.allium`; finding 12 and notes N27, N04 and N16 of
   `double/SPEC-NOTES.md`; `crates/libpachinko/src/queue/pockets.rs`, `queue/pull.rs`,
   `queue/inspect.rs`; `double/pachinko_double/pull.py`, `publish.py`, `settle.py`,
   `expiry.py`, `queue.py` and `invariants.py`; and the tickets named under "Other
   tickets this one touches", as merged.
2. **Re-verify Context.** Check every statement under "What exists today" against the
   tree and record which held and which moved. Confirm the worked lapse case by reading
   `LeaseExpiryParksPocket`, `LeaseExpiryKeepsPocketAtFront` and
   `LeaseExpiryIdlesEmptyPocket`. Repeat the search for `position` over
   `crates/pachinko`. Read the collection types and operations in
   `allium-skill-reference/allium/language-reference.md` and record what Allium offers
   for an ordered collection, with the section each fact came from.
3. **The comparison.** Define each model compared (the four candidates, or the
   executor's revision of them) and, for each, write down:
   - the entities, fields, rules, invariants and vocabulary entries it adds, changes or
     removes, by name;
   - what `Queue.next_position` still allots, and what that does to records' absolute
     positions and to T47's first position;
   - whether a reader or an implementation can still ask "where does this pocket stand"
     for a queued, a leased, an idle and a parked pocket, and what the answer is;
   - the pocket order after each worked case: a first publish; a release with records
     left; a release that idles and the revival after it; a parking and the revival
     after it; one lapse; the two-lapse case of Context, in both orders; a lapse while
     another pocket was queued later;
   - whether any of those orders differs from today's, stated as a difference, never
     folded into the model;
   - what it asks of the Rust (`Pocket.position`, `Pockets.order`, `PocketView`,
     `Snapshot.pocket_order`, the fixture's three checks, the walk model), of the double
     and of the catalog, and whether the service changes at all;
   - what it gives T39's `new_pocket_first` to build on, and what it does to the finding
     6 comment;
   - where it leaves a leased pocket if T42 changes what a lapse does, since a leased
     pocket's place matters only because a lapse returns the pocket to it;
   - what it would cost a later write-ahead log and recovery to restore the order, as
     far as can be said without designing one (the maintainer raised recovery under
     finding 13, about the counter's starting value).

   State a preferred model and the reasons. The scratch work goes in `ai_tmp/t40/`.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint T40-1, over the comparison,
   since the draft and the record build on it. Ask: is any model's order in a worked
   case wrong against the rules; is a behaviour difference hidden or called none; does
   any statement about Allium lack a section of the language reference; does any claim
   about the Rust or the double lack a file and item; has the scope reached records; is
   the leaning ("clear it on idling") presented as a ruling, or extended to parked
   pockets without the maintainer; is a candidate dismissed without a worked case.
4. **The draft specification, checked and then removed.** Use the `spec-change` skill
   for the drafting only. Apply the preferred model to `docs/specs/pachinko.allium` in
   the working tree, uncommitted. Run `just check-specs` and `just analyse-specs` and
   quote their output. Save the draft with
   `git diff -- docs/specs/pachinko.allium >| ai_tmp/t40/draft.patch`, confirm the file
   is not empty and that `git apply --check -R ai_tmp/t40/draft.patch` exits 0, and then
   remove the draft with `git apply -R ai_tmp/t40/draft.patch`. That reverses exactly
   what the patch holds and discards nothing else; never use `git restore` or
   `git checkout` for it. `git diff --stat -- docs/specs` then prints nothing. The patch
   is scratch: the decision record of step 5 carries the checked draft in full, and it
   is the record, not the patch, that a follow-up ticket works from.

   If the pinned checker rejects the preferred model in a way the model cannot be
   reworded around, that is a finding for the comparison: record the diagnostics, say
   which model can be stated, and carry both to step 6. If the leaning's model (B) is
   not the preferred one, draft it as well, by the same procedure and to its own patch
   file, so the maintainer sees both in Allium.

   Unverified when the ticket was written: `just check-specs` runs `bg-run-allium check`
   over the modules under `docs/specs/`, and no recipe checks a file elsewhere. That is
   why the draft is applied in place. If a recipe for a scratch copy exists by then, use
   it and leave the module untouched.
5. **The decision record.** Use the `review-docs` skill. Take the next free number
   (re-read `docs/decisions/README.md`; another ticket may have taken 0021). Write the
   record in the shape of `docs/decisions/0020-held-set-until-lease-ends.md`: context,
   with the maintainer's question as this ticket records it and the date; the models; the worked
   cases; the recommendation, marked **proposed, not yet ruled**; the checked draft as
   fenced Allium, every declaration and comment the patch changes, whole, so that the
   record alone is enough to reapply it; the consequences, including the ones that
   hurt; what would reopen it. Add its row to `docs/decisions/README.md`, its entry to
   `docs/manifest.yml`, and a link from `docs/README.md` only if the gate asks for one.
   Run `just check-docs`. Commit.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint T40-2, over the record and
   the patch files, before the maintainer is asked. Ask: does the record's draft equal
   the checked patch; does the record say "proposed" everywhere a reader could take it
   for decided; is every open point below carried into what the maintainer will be
   asked; does the record restate another page's topic instead of linking it; did
   anything outside Files touched change; do `docs/specs`, `crates` and `double` differ
   from the base.
6. **Authorisation required: the maintainer's ruling.** Give the maintainer the record,
   the comparison and the review's open findings, and ask:
   - which model; and for each case where the recommended model orders pockets
     differently from today, whether that difference is accepted;
   - each open point below, in order;
   - whether T39 and T47 keep `depends_on: T40` alone or also wait on the follow-up
     ticket that changes the specification, and whether this ticket may edit their
     `depends_on` and add a dated note to each;
   - the number and split of the follow-up tickets step 7 proposes.

   Then stop and do nothing further until the maintainer answers. Quote the answers in
   the hand-back, dated. If the maintainer picks a model other than the recommended
   one, draft it by step 4's procedure, revise the record, and stop again with the
   checked draft. An answer that names something outside the models is a new design:
   record it and stop again.
7. **Record the ruling and write the follow-up tickets.** Amend the decision record: the
   ruling, its date, the maintainer's reason in the maintainer's words, and the
   recommendation's "proposed" replaced by what was decided. Then write the follow-up
   tickets, each to `CONVENTIONS.md` §11 and §14 and in the shape of
   `tickets/T39-decide-q2-q6.md`, self-contained, at the next free ids (re-read
   `tickets/`; T42 and T43 create tickets too). Between them they cover:
   - the specification: the ruled model in `docs/specs/pachinko.allium`, from the
     checked draft in the decision record, with the vocabulary comment, the Pocket and Queue comments, the two
     pocket-position invariants and the finding 6 comment as the ruling leaves them;
   - the Rust library, its tests, the fixture's checks, the walk model and the
     inspection seam;
   - the service, only if the ruling reaches it;
   - the Python double, its tests, `double/README.md` and `double/SPEC-NOTES.md`
     (finding 12 and N27 marked answered and applied);
   - the catalog and the handbook: `docs/reference/testing.md`,
     `docs/project/terminology.md`, `docs/explanation/architecture.md` and any other
     page step 2 found.

   The specification moves first; code, tests and pages follow in the same ticket or in
   tickets that depend on it, so that the specification and the code agree at every
   merge. If the ruling is model A with no change, no implementing ticket is written:
   say so, and name the one-line follow-up that marks finding 12 answered. Add each
   ticket's row to `tickets/README.md` and place it in the graph. Edit T39 and T47 only
   as step 6 authorised.

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
   touches in the model: the section that holds the pocket order, the effects that
   state a position (a claim's, a release's and a revival's), the two pocket-position
   invariants, and `pocket_order` in each replay's list of what it does not compare.

   **Adversarial review** (`CONVENTIONS.md` §14), checkpoint T40-3, over the new tickets
   and the amended record. Ask: can each ticket be run by an agent with no context; does
   any cite a chat, a scratch path or this ticket's hand-back for a fact it needs; does
   each restate the ruling with its date and reason; is any update the ruling needs in
   no ticket; do two tickets list the same path without saying who goes first; does any
   ticket decide something the maintainer left open.
8. **Close.** Use the `project-check` skill. Run `git fetch origin` (network, authorised
   by this step) and merge the base step 0 named (`origin/main` unless the maintainer
   named another). Rerun the Verification block at the resulting
   commit and requote it. Set `status: done` here and in `tickets/README.md`, write the
   hand-back notes, each quoted output headed by the commit it ran at, and commit.

   **Authorisation required:** pushing the branch and opening the pull request are each
   asked for separately.

## Acceptance criteria

- The baseline `just check` line is quoted, and step 2's record says which statements
  of Context held.
- The comparison defines each model and gives, for each, every item step 3 lists, with
  the pocket order after every worked case. Each difference from today's order is
  stated as one.
- The preferred model's draft passed `just check-specs` and `just analyse-specs`, with
  the output quoted, or the diagnostics that stopped it are quoted and the comparison
  says what follows.
- `git diff --stat "${base}...HEAD" -- docs/specs crates double` prints nothing, with
  `base` as the Verification block sets it.
- The decision record exists at the next free number, is registered in
  `docs/manifest.yml` and `docs/decisions/README.md`, and `just check-docs` passes.
- The maintainer's ruling and the answer to each open point are quoted in the hand-back
  and in the record, dated, and nothing before step 6 chose an option.
- The leaning is recorded as a leaning for the position-on-Pocket branch only.
- Each follow-up ticket exists, is self-contained, restates the ruling, and has its row
  in `tickets/README.md`; or the hand-back says why none was needed.
- T39 and T47 were edited only as the maintainer authorised at step 6.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed
  back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
line that must print nothing follows its positive control. `base` is `origin/main`, or
the base step 0's stop named.

```sh
set -o pipefail
base=origin/main
# positive control: the module is read and the one rule that reads a pocket's position is found
rg -n 'min_by\(free, p => p\.position\)' docs/specs/pachinko.allium; echo "exit $? (must be 0)"
# positive control: the branch changed files; they are under docs/decisions, docs/manifest.yml,
# docs/README.md and tickets/ only
git diff --name-only "${base}...HEAD"; echo "exit $?"
# the specification, the crates and the double are unchanged: must print nothing
git diff --stat "${base}...HEAD" -- docs/specs crates double; echo "exit $?"
git status --short -- docs/specs crates double; echo "exit $? (must print nothing)"
# the record is registered: each must match
ls docs/decisions/*-pocket-order.md; echo "exit $?"
rg -n 'pocket-order\.md' docs/manifest.yml docs/decisions/README.md; echo "exit $?"
# scratch work is ignored
git check-ignore -v ai_tmp/t40; echo "exit $? (must be 0)"
just check-docs; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $? (must print nothing)"
```

At step 4, with the draft applied and before it is removed:

```sh
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
```

## Open points

Each is put to the maintainer at step 6 and none is assumed.

1. **Which model.** The ruling this ticket exists to get.
2. **Parked pockets, under the leaning.** The maintainer selected "clear it on idling".
   A parked pocket's position is as unread as an idle one's; whether it is cleared too
   was not asked.
3. **A change of order.** Whether any difference the recommended model makes to the
   pocket order, in the two-lapse case or elsewhere, is accepted.
4. **The shared counter.** If pockets stop taking values from `next_position`, whether
   records' absolute positions may shift, and what that means for the first position
   T47 settles.
5. **The inspection seam.** Whether `PocketView.position` and `Snapshot.pocket_order`
   in the library's `testing` module follow the specification's model or stay as the
   implementation's own view.
6. **T39 and T47.** Whether they wait on this ticket alone or on the follow-up that
   changes the specification.
7. **The base.** Whether this ticket runs from `main` once `build-python-double` has
   merged, or from that branch (step 0).

## Hand-back notes

### Baseline

### Context re-verified (step 2)

### The comparison (step 3) and T40-1

### The draft and the checker's output (step 4)

### The decision record (step 5) and T40-2

### The maintainer's ruling (step 6)

### The follow-up tickets (step 7) and T40-3

### Verification
