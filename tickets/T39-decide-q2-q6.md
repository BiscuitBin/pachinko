---
id: T39
title: "Land the operator listing of the shed queues (question 2) and new-pocket-first (question 6), both ruled"
status: todo
depends_on: [T35, T40]
parallel_with: []
branch: ticket/t39-decide-q2-q6
estimated_size: L
---

# T39: Land the operator listing of the shed queues (question 2) and new-pocket-first (question 6), both ruled

## Context

`docs/specs/pachinko.allium` ends with two `open question` lines, questions 2 and 6 in
the module's numbering, which never compacts:

- Question 2: "Who reads the superseded and dead-letter queues, and what can they do: an
  operator listing, a replay action back into the pocket, or a consumable stream under
  the lease model? The answer also fixes what an entry must carry; reason and nacked_by
  are kept on that assumption."
- Question 6: "Should a newly created pocket be able to jump to the front of the queue,
  opt-in, for fast first service?"

The maintainer ruled on all six questions the module then carried, on 2026-11-23 and
2026-11-24. Four rulings changed only the module's wording and are in it already:
questions 3, 4, 5 and 7 are closed, and the module's Open Questions comment records
each. The rulings on questions 2 and 6 add behaviour that no implementation has. The
maintainer chose to land those two by ticket, with their code and tests, so that the
specification and the code agree at every commit. This is that ticket. Until it lands,
both lines stay in the module, and the code answers neither.

Nothing in this ticket re-opens a ruling. A finding that a ruling is wrong goes to the
maintainer as a question, never into a fix.

### The ruling on question 6: `new_pocket_first` (2026-11-23, details 2026-11-24)

A per-deployment switch puts every newly created pocket ahead of all others.

- **Why.** The key case is a newly onboarded tenant. An existing tenant already has data
  to use the product with. A new one has nothing to do until its first pass through the
  queue, so first service decides its time to value.
- **The switch.** `config.new_pocket_first: Boolean = false`. Off is today's behaviour
  exactly.
- **Only a first creation jumps.** The pocket `PublishIntoNewPocket` creates jumps. A
  revived idle or parked pocket goes to the back, as today.
- **New pockets among themselves.** They queue oldest first, ahead of every other
  pocket.
- **Starvation is accepted, and the specification says so.** With the switch on, other
  pockets are claimed only when no new pocket is waiting.
- **After the first lease, today's rules apply.** A release with records left gives the
  pocket a fresh position at the back. A lapse keeps the pocket's standing and retries
  it, and it parks at the threshold.
- **A named consequence.** With the switch on, a lapsed ordinary pocket keeps its
  position but is retried behind every waiting new pocket. Two texts in the module then
  say too much and need rewording: the Lease expiry comment's "the pocket keeps its
  place at the front", which `LeaseExpiryKeepsPocketAtFront` is named for, and
  `ExpiryRecoversWithinTtl`'s "pullable again at once".

**A suggested model, for this ticket's executor to confirm with the maintainer at step
1, not a ruling.** A Boolean on `Pocket` for its first-service standing, which
`PullClaimsFrontPocket` prefers and which a release, idling and a revival clear.
Positions stay one rising counter. The field is not called a "mark": in the module a
mark is an ack or a nack. Under this model the counts move: six config defaults where
there are five, and one more invariant than today's 35.

### The ruling on question 2: an operator listing (2026-11-24)

An operator lists both shed queues, read-only. There is no replay and no consuming.

- An operator reads `SupersededRecord` and `DeadLetter` entries, and nothing the
  operator does changes state.
- `SupersededRecord.reason` and `DeadLetter.nacked_by` stay, since the listing shows
  them. The question's last sentence is answered by that.

The ruling leaves four things undecided. Step 1 puts each to the maintainer; none is
assumed:

1. The listing's order. One thing about it is ruled (2026-11-24, finding 8 of
   `double/SPEC-NOTES.md`): the listing must not present `shed_at` order as version
   order. `shed_at` is the time a version was shed and does not order a key's versions;
   `rank` does, as the comment on `entity SupersededRecord` says.
2. Whether a listing covers the whole deployment or one pocket.
3. How an Operator exists in the specification: an `actor`, or an entity. The module's
   `Publisher` and `Consumer` are entities that a join creates, and the maintainer ruled
   on 2026-11-24 that joining is dropped (`tickets/T45-drop-joining.md`), so an Operator
   that joins is not a model to copy. Read what T45 has merged before asking.
4. The route's transport and authentication under decision 0019
   (`docs/decisions/0019-service-runtime-tokio-and-http.md`), which records that the
   service has no authentication and listens on loopback by default.

### Tickets written after this one

On 2026-11-24 the maintainer ruled on the findings of `double/SPEC-NOTES.md`, and
tickets T40 to T47 carry the rulings that add behaviour. Four of them touch this one.
Re-read what each has merged at step 3:

- **T40** (`tickets/T40-pocket-order-model.md`) decides whether the pocket order
  belongs on the Pocket or on the Queue. This ticket depends on it, so that
  `new_pocket_first` is designed once, on the model that wins. The suggested model above
  assumes pocket positions as they are today.
- **T42** (`tickets/T42-clock-model.md`) owns the Lease expiry comment and may change
  what a lapse does. This ticket's rewording of that comment and of
  `ExpiryRecoversWithinTtl`, and its "a lapse keeps the pocket's standing", start from
  whatever T42 has merged.
- **T45** (`tickets/T45-drop-joining.md`) drops joining, which changes undecided point
  3 above.
- **T46** (`tickets/T46-no-claim-without-records.md`) changes scenarios S031 and S032.
  "Every existing scenario stays green unchanged" below means the scenarios as they
  stand on `main` when this ticket starts.

### The four closures already in the module

For orientation only; this ticket changes none of them.

| Question | Closed | Ruling |
| --- | --- | --- |
| 5, an adaptive lease TTL | 2026-11-23 | `lease_ttl` stays one fixed config value. It is a liveness timeout: a live consumer renews, and a deployment that wants fast recovery sets a short TTL. |
| 7, reviving a parked pocket | 2026-11-23 | Only a publish revives a parked pocket. No operator action, no cooldown. |
| 4, routing by worker pool or priority | 2026-11-24 | Worker pools and priority classes run as separate deployments. Its "newly onboarded tenants first" is the subject of question 6. |
| 3, capacity | 2026-11-24 | For capacity, superseded records go first, oldest `shed_at` first; dead letters only when no superseded record is left, oldest `nacked_at` first. A `Record` is never removed or bounded, and a publish is never refused. |

The ruling on question 7 bounds this ticket: the operator of question 2 gets no action
that revives a parked pocket.

### What exists today

Re-read each at step 3; these were true when the ticket was written.

- The module's `config` block has five values, and `crates/libpachinko/src/config.rs`,
  `crates/pachinko/src/config.rs` (the `serve` flags) and
  `docs/reference/configuration.md` carry the same five.
- `PullClaimsFrontPocket` claims `min_by(free, p => p.position)` over the queued
  pockets, and `PublishIntoNewPocket` creates a pocket at `queue.next_position`.
- No surface exposes the shed queues. In the library, `SupersededRecord`, `DeadLetter`,
  `ShedReason` and `PocketStatus` are exported by the `testing` module alone; four
  `#[expect]` reasons in `crates/libpachinko/src/queue/types.rs` say "until open
  question 2 is answered", and `crates/libpachinko/src/lib.rs` and
  `crates/libpachinko/src/queue/inspect.rs` say the same in prose.
- `docs/reference/http-api.md` lists the routes; none reads a shed queue, and its text
  on superseded records and dead letters says "No route reads them".
- `docs/reference/testing.md` holds 93 scenarios, S001 to S093, so the next free id is
  S094. No row carries an `oq:N` tag.
- A Python double of the library, `double/`, with its notes in `double/SPEC-NOTES.md`,
  exists on the branch `build-python-double`. It was not on `main` when this ticket was
  written.

### Risks (`CONVENTIONS.md` §13)

- **The specification moves first** (`AGENTS.md`, invariant 1). The rules for both
  rulings land, and pass `just check-specs` and `just analyse-specs`, before any test
  or code.
- **Off must be today's behaviour exactly.** Every existing scenario runs with
  `new_pocket_first` at its default and stays green unchanged. A test edited to pass is
  a finding.
- **Starvation is by design.** A test that pins it is correct. A fairness property in
  the invariant walk that assumes every queued pocket is eventually claimed holds only
  with the switch off.
- **A public surface grows.** The library gains a config field and a way to read the
  shed queues outside `testing`; `just semver` reports it, and each new public item
  keeps invariant 3 and has a doctest.
- **The library and the service land together.** A config field the service cannot set,
  or a listing the library exposes and no route reads, is half a ruling.
- **A read that changes state.** The listing is read-only: it never removes an entry,
  resets a TTL or touches a pocket. A test shows the snapshot unchanged after a listing.
- **An unauthenticated operator route.** Whatever step 1 decides for item 4 is stated in
  `docs/explanation/security-model.md` in the same commit as the route.
- **The small denominator.** Each new branch has its test, or coverage falls below 90%.
- **lychee and typos over `tickets/`.** The hand-back notes write paths and test names
  as code spans, never as links.

## Goal

The specification, the library, the service, the tests and the handbook say and do what
the two rulings say, and the module carries no `open question`:
`rg -c '^open question' docs/specs/pachinko.allium` finds none. With
`new_pocket_first` off, nothing a caller can observe has changed. With it on, a pocket
created by a first publish is claimed before every other pocket, oldest new pocket
first. An operator can list the superseded records and the dead letters, and listing
changes nothing.

## Non-goals

- No replay of a shed entry into its pocket, and no consuming of the shed queues under
  the lease model.
- No operator action that revives a parked pocket, and no cooldown (question 7's
  ruling).
- No capacity reclaim. Question 3's ruling states the order; removing entries early is
  not this ticket's work.
- No per-pocket or per-publish opt-in for jumping the queue: the switch is one value per
  deployment.
- No bound on starvation with the switch on.
- No change to how a revived, released or lapsed pocket is positioned with the switch
  off.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | The `new_pocket_first` config value and its rules; the operator's read of the shed queues; the reworded Lease expiry comment and `ExpiryRecoversWithinTtl`; both `open question` lines removed and the Open Questions comment extended; the header's Excludes line on reading the shed queues |
| `crates/libpachinko/src/config.rs`, `queue.rs`, `queue/pockets.rs`, `queue/pull.rs`, `queue/publish.rs`, `queue/types.rs`, `queue/inspect.rs`, `lib.rs`, `testing.rs` | shared, own lines | The config field, the claim order, the listing, and the exports the four `#[expect]` reasons were waiting on |
| `crates/libpachinko/tests/queue/*.rs`, `crates/libpachinko/tests/queue/walk/model.rs` | shared, own lines | The new scenarios, the walk model and the invariant checks |
| `crates/pachinko/src/config.rs`, `http.rs`, `http/*.rs`, `actor.rs`, `error.rs` | shared, own lines, new as step 3 decides | The flag for the switch, and the listing's route as step 1 decides it |
| `crates/pachinko/tests/http.rs` | shared, own lines | The wire drivers for the new `wire` scenarios |
| `docs/reference/testing.md`, `docs/reference/http-api.md`, `docs/reference/api.md`, `docs/reference/configuration.md` | shared, own lines | The catalog rows and counts; the route; the flag |
| `docs/how-to/work-with-the-specs.md`, `docs/explanation/specifications.md`, `docs/explanation/prior-art.md`, `docs/explanation/security-model.md`, `docs/project/purpose-and-scope.md`, `docs/project/terminology.md` | shared, own lines | The open-question counts and lists, which fall to none; the closures; the operator route's exposure; any new term |
| `README.md`, `crates/libpachinko/README.md`, `CHANGELOG.md` | shared, own lines | "who reads them is an open question"; the signatures; both Unreleased sections |
| `double/**` | shared, own lines, if the base holds it | The double's config, rules, tests, guide and `double/SPEC-NOTES.md`, whose "Open questions left untouched" section then lists none |
| `quint/pachinko.qnt`, `quint/verify.txt`, `quint/mutants/*.patch`, `quint/traces/*`, `scripts/quint-traces.py`, `crates/libpachinko/tests/queue/walk/replay.rs`, `crates/libpachinko/tests/queue/walk/model/trace.rs`, `double/tests/test_replay.py` | shared, own lines, if the tree this ticket lands on holds `quint/pachinko.qnt`, at the base or after the closing merge or rebase, and each path only where that tree holds it | `new_pocket_first` in the configuration, with an instance that sets it and its line in `quint/verify.txt`; the standing step 1 ruled for a new pocket, in `PublishIntoNewPocket`, in the claim's choice of pocket and in each rule that clears it; the properties that transcribe the reworded Lease expiry comment and `ExpiryRecoversWithinTtl`, where the model has them; each mutant patch the edit breaks, re-cut, and one new mutant on the claim of a new pocket; the trace fixtures, regenerated if they exist, with their generator and the two replays that read them |
| `tickets/README.md` | shared, own lines | T39's Status cell |
| `tickets/T39-decide-q2-q6.md` | ticket | `status:` and hand-back notes |

## Steps

1. **Authorisation required: what the rulings leave open.** Ask the maintainer, quoting
   this ticket's Context:
   - The four undecided points of the listing, in the order Context numbers them.
   - Whether the suggested model for `new_pocket_first` stands: the Boolean on `Pocket`,
     what clears it, and its name.
   - What `new_pocket_first` means for a pocket that already exists when a deployment
     turns the switch on, if the model leaves that open.
   - The names: the flag, the route and the library's listing.
   - The base branch. The double lives on `build-python-double`; if this ticket's base
     does not hold `double/`, say so and ask whether the double moves here or in a
     follow-up.

   Then stop until the maintainer answers, and quote the answers in the hand-back,
   dated. An answer that names something outside these points is a new design: record
   it and stop again.
2. **The worktree.** Create the branch and its worktree from the base step 1 named
   (`CONVENTIONS.md` §11). Run `just initialize` (network, authorised by this step). Run
   `just check` before any edit and quote its last line as the baseline.
3. **The plan.** Use the `plan-change` skill. Read the whole module and
   `just plan-spec pachinko`'s output, and re-read everything "What exists today"
   names. Write down, before any test: the rules to add and change; the scenarios, from
   S094; the store and queue changes; the route and its errors; each page to change.
4. **The specification.** Use the `spec-change` skill. Land both rulings as rules, with
   step 1's answers; reword the Lease expiry comment and `ExpiryRecoversWithinTtl`;
   remove both `open question` lines; extend the Open Questions comment with each
   closure and its date. Run `just check-specs` and `just analyse-specs`, then
   `just plan-spec pachinko` for the new obligations.

   **Adversarial review** (`CONVENTIONS.md` §14): the specification diff against the
   rulings above and step 1's answers. Does off equal today's rules? Does any rule let a
   listing change state? Is the starvation stated?
5. **The tests, red first.** Derive each scenario and obligation test from the changed
   clauses and show each failing before the code changes; quote the failures.
6. **The code.** Use the `rust-change` skill. The library first, then the service, in
   one commit or in commits that each leave `just check` green. Run `just fmt-check`,
   `just clippy`, `just test`, `just coverage`, `just doc`, `just effects` and
   `just semver` as the change needs them.
7. **The double**, if the base holds it: the same rules, its tests, its guide and its
   notes, with `just python-double` green.

   **The Quint model**, if the tree this ticket lands on holds `quint/pachinko.qnt`, at
   the base or after the closing merge or rebase onto `main`: that model is derived
   from this specification, so the same change updates it wherever it models what this
   ticket changes. Read the condition again after that merge: if it brought in the
   model, `quint/traces/` or a replay the base did not hold, this paragraph is done
   then, before any gate runs, each stop in it included. The rule is the same-change
   rule in `AGENTS.md` and the decision record under `docs/decisions/` that introduced
   the model; both exist wherever the model does.
   Here that is more than the publish: `new_pocket_first` in the
   configuration; the standing step 1 ruled for a new pocket, where
   `PublishIntoNewPocket` gives it, where the claim prefers it and where a release,
   idling and a revival clear it; and, where the model already has them, the properties
   that transcribe the Lease expiry comment and `ExpiryRecoversWithinTtl`, reworded as
   step 4 rewords the module. The operator's listing reads state and changes none.

   Both values of the switch are checked, or the model has not followed the ruling: a
   model that carries the field and still claims by position alone must not pass. So
   `just verify-quint` covers an instance with `new_pocket_first` on beside the one
   with it off, a run on that instance reaches a new pocket claimed ahead of an older
   one, and one new mutant on that path (the claim ignores the standing) is caught by
   the property the patch names. If a second instance does not fit the verify job's
   timeout, **stop and ask**; never drop an instance or shrink a domain.

   That check is owed only where the model has `PublishIntoNewPocket`. An earlier
   model has the claim and no publish, so no rule gives a pocket the standing and no
   run can reach a new pocket: there, change only what a rule the model already has
   reads, leave the rest to the model's own ticket for publish, which reads this
   specification at its base, and say in the hand-back that the check of both values
   was not owed and why. If the model has `PublishIntoNewPocket` and the tree has no
   `just verify-quint` recipe, **stop and ask** how the check is to be shown.

   In this order: edit the model; re-cut each mutant patch under `quint/mutants/` that
   the edit breaks, keeping its mutation and the property it names, and list each in
   the hand-back; if `quint/traces/` exists, run `just quint-traces` to regenerate it,
   changing `scripts/quint-traces.py` where a witness or the start changes, and keep
   each replay that exists green, the Rust test under
   `crates/libpachinko/tests/queue/walk/` in `just test` and
   `double/tests/test_replay.py` in `just python-double`, where a field or a step the
   model gained is either mapped or added to that replay's own list of what it does not
   compare, with the ticket that will map it, and a step only with the maintainer's
   answer on which ticket that is; then run `just check-quint`, and
   `just verify-quint` if the recipe exists. The fixtures come before the two gates
   because both fail on a model whose hash the fixtures do not record. Where the
   model's header lists the behaviour under "Not yet modelled", change nothing there
   and say so in the hand-back.
8. **The handbook.** Use the `review-docs` skill for every page in Files touched. Run
   `just check-docs`.
9. **Close.** Set `status: done` here and in `tickets/README.md`, and write the
   hand-back notes.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Then run
   `just check`.

   **Authorisation required:** pushing the branch and opening the pull request are each
   asked for separately.

## Acceptance criteria

- The maintainer's answers at step 1 are quoted in the hand-back with their date, and
  nothing before them chose an option.
- `docs/specs/pachinko.allium` carries no `open question` line, and its Open Questions
  comment records the closures of questions 2 and 6 with their dates.
- With `new_pocket_first` off, every scenario from S001 to S093 passes with its test
  unchanged.
- With it on: a pocket created by a first publish is claimed before every older queued
  pocket; two new pockets are claimed oldest first; a revived idle pocket and a revived
  parked pocket go to the back; a released new pocket with records left goes to the
  back; a lapsed new pocket keeps its standing and parks at the threshold; a lapsed
  ordinary pocket is retried behind a waiting new pocket. Each is pinned by a tagged
  library test that was red before the code changed.
- The module states that other pockets are claimed only when no new pocket is waiting.
- An operator can list the superseded records and the dead letters as step 1 decided,
  each entry showing `reason` or `nacked_by`; a test shows the state unchanged after a
  listing; no route or method replays or consumes an entry.
- No `#[expect]` reason, doc comment or page says question 2 or question 6 is open.
- Every `// obligation:` id in `crates/` is in `just plan-spec pachinko`'s output or
  names a `@guarantee` of the module, which the planner gives no id
  (`docs/reference/testing.md`).
- The scenario ids run contiguously from S001 to the catalog's last row, each tagged
  once, and the catalog's counts of scenarios, invariants and config values match the
  module.
- Every new or changed public item keeps invariant 3 and has a doctest; `just coverage`
  reports at or above 90%; `just effects` passes; the `just semver` report is quoted.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
search that must find nothing passes only on `rg`'s status 1, never on an error, and
follows a positive control over the same files. The searches span lines (`-U`) and a comment's line marker,
since the prose wraps. At step 2, before any edit, the stale-question search is run once and
must match, in `crates/libpachinko/src/testing.rs` among others: that shows it can.

```sh
set -o pipefail
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
# positive control: the file is read and its Open Questions heading is found (exit 0)
rg -c '^-- Open Questions' docs/specs/pachinko.allium; echo "exit $?"
# no open question line is left: rg must exit 1, so this prints "exit 0"
rg -n '^open question' docs/specs/pachinko.allium; [ "$?" -eq 1 ]; echo "exit $?"
# positive control: the same trees are read and the wrapped phrase is found (exit 0);
# add double to both searches if the base holds it
rg -c -U -i 'open(\s|//[/!]?)+question' \
  crates docs/reference docs/explanation docs/project docs/how-to README.md; echo "exit $?"
# no page, comment or reason calls question 2 or 6 open: rg must exit 1
rg -n -U -i 'open(\s|//[/!]?)+question(\s|//[/!]?)+(2|6)\b|(second|sixth)(\s|//[/!]?)+open(\s|//[/!]?)+question' \
  crates docs/reference docs/explanation docs/project docs/how-to README.md
[ "$?" -eq 1 ]; echo "exit $?"
just test; echo "exit $?"
just coverage; echo "exit $?"
just effects; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Hand-back notes

None yet.

## Open points

- The four undecided points of the listing and the model for `new_pocket_first` are
  step 1's to settle.
- Whether the double moves in this ticket depends on the base step 1 names.
