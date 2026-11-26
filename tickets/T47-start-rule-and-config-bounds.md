---
id: T47
title: "State the start of a deployment as a rule that requires the config bounds and creates the Queue, and settle the Queue's first position"
status: todo
depends_on: [T40, T43]
parallel_with: []
branch: ticket/t47-start-rule-and-config-bounds
estimated_size: M
---

# T47: State the start of a deployment as a rule that requires the config bounds and creates the Queue, and settle the Queue's first position

## Context

`docs/specs/pachinko.allium` states five bounds on its `config` values as invariants,
each under a comment that begins "The model is defined for". It does not say what
happens to a deployment configured outside those bounds. It also declares the
deployment's one `Queue` in a `given` block, so the Queue simply exists, and its counter
`next_position` has no stated first value.

Both implementations had to choose. The Rust library refuses a zero limit when a queue
is built and makes a negative TTL impossible to write. The Python double refuses all
five when a `Config` is made. Both start the counter at 0, and six catalog rows assert
absolute positions that hold only for that start.

`double/SPEC-NOTES.md` records these as finding 14 (note N09) and the first-position
part of finding 13 (notes N04 and N16). The maintainer ruled on both on 2026-11-24. This
ticket lands the rulings: the specification first, then the Rust library, the service,
the double, its notes and the catalog.

Nothing in this ticket re-opens a ruling. A finding that a ruling is wrong goes to the
maintainer as a question, never into a fix.

### The ruling on finding 14 (N09), 2026-11-24

The question was: is "refused at configuration" the intended reading of "the model is
defined for", or should the specification state the refusal itself?

The ruling: **model the refusal as a rule.** Starting a deployment is a trigger. The
rule's `requires` are the five config bounds, and the rule creates the Queue. This adds
behaviour to the specification, so it lands by ticket, with its code and tests.

The ruling leaves three things to this ticket:

1. **The refusal's name.** Finding 5 of the same notes ruled that the specification
   names its refusals and fixes the order they are checked, and T43
   (`tickets/T43-caller-contract.md`) works that out, including the name of the start
   rule's refusal of a config outside its bounds. This ticket takes the name, and how
   Allium states a named refusal, from T43's merged decision. It invents neither.
2. **The first position the rule gives the Queue.** See the next ruling.
3. **Whether the Rust's compile-time refusal of a negative `Duration` satisfies a rule
   that refuses at run time.** Scenario S005 is a `compile_fail` doctest in the Rust.
   This is an open point, put to the maintainer at step 3.

### The ruling on finding 13's first position (N04, N16), 2026-11-24

The question was: is the Queue's first position meant to be unspecified, given that six
rows assert absolute positions that hold only because both implementations start at 0?

The ruling: **the first position (unspecified, or 0) is an open point of this ticket,
settled after the pocket-order decision of T40** (`tickets/T40-pocket-order-model.md`).
T40 asks whether the pocket order belongs on the `Pocket` or on the `Queue`, and its
outcome may change what a position is. The maintainer's note: once a write-ahead log and
recovery are added, the starting value may matter, depending on what that ticket
decides. No such ticket exists today.

Until this ticket settles the point, the six rows S011, S012, S014, S015, S016 and S022
stay `impl-choice` in the double. Step 3 puts the point to the maintainer; it is not
assumed.

### What other tickets decide for this one

- **T43** decides the refusal's name and how Allium states a named refusal. Step 1 reads
  T43's merged decision record and stops if it is absent or does not name this refusal.
- **T40** decides the pocket-order model. Step 1 reads its merged decision record and
  the follow-up tickets it created. If a follow-up ticket that changes `Queue`,
  `next_position` or `Pocket.position` has not merged, step 1 says so and asks the
  maintainer whether this ticket waits for it.
- **T39** (`tickets/T39-decide-q2-q6.md`) adds a sixth config value,
  `new_pocket_first: Boolean = false`. A Boolean has no bound, so the bounds stay five
  while the config values become six. T39 and this ticket are not ordered: whichever
  lands second re-reads the other's merged text and counts again.
- **T42** (`tickets/T42-clock-model.md`) explores the clock model. One model it weighs
  holds the latest time as Queue state. If that has merged, the start rule gives the new
  field its first value; if it merges later, its ticket extends the start rule. Step 1
  reads what has merged and says which.
- **T45** (`tickets/T45-drop-joining.md`) removes joining, and T39 asks whether an
  Operator is an entity or an `actor`. Both bear on who starts a deployment (open point
  4 below). Step 1 reads what has merged.

### What exists today

Re-read each at step 2; these were true when the ticket was written, on the branch
`build-python-double`.

The specification, `docs/specs/pachinko.allium`:

- `given { queue: Queue }` at lines 92 to 94, under the comment "The deployment's one
  queue".
- `entity Queue { next_position: Integer }` at lines 107 to 109. Its comment (lines 100
  to 106) says every rule that allots a position takes `next_position` and advances it.
  No rule creates a `Queue`, and nothing gives `next_position` a first value.
- The `config` block at lines 292 to 298, five values: `lease_ttl: Duration = 5.minutes`,
  `max_consecutive_expiries: Integer = 3`, `max_pull_batch: Integer = 100`,
  `superseded_ttl: Duration = 12.hours`, `dead_letter_ttl: Duration = 12.hours`.
- The five config invariants, each a top-level `invariant`:
  - `ExpiryLimitPositive`, lines 972 to 974, `config.max_consecutive_expiries >= 1`; its
    comment (969 to 971) begins "The model is defined for max_consecutive_expiries >= 1".
  - `PullBatchPositive`, lines 979 to 981, `config.max_pull_batch >= 1`; its comment
    (976 to 978) begins "The model is defined for max_pull_batch >= 1".
  - `LeaseTtlNonNegative` (994 to 996), `SupersededTtlNonNegative` (998 to 1000) and
    `DeadLetterTtlNonNegative` (1002 to 1004), each `config.<ttl> >= 0.seconds`. One
    comment (983 to 993) covers the three and begins "The model is defined for
    non-negative TTLs". It says a zero TTL is admitted, and ends "A positive floor is
    the maintainer's to set."
- The module has 35 invariants: 20 at the top level and 15 inside entities.
- `PocketPositionsBelowNext` (line 1068), `RecordPositionsBelowNext` (line 1073) and
  `ArrivalPositionsBelowNext` (line 1099) keep every position below
  `queue.next_position`. Nothing bounds a position from below.
- The header's Excludes list (line 20) says "Storage, snapshots, replication and
  recovery: implementation."
- Two surfaces, `Publishing` (line 1109) and `Consuming` (line 1150). Neither provides
  a trigger that starts a deployment.

The Allium language reference, `allium-skill-reference/allium/language-reference.md`:

- "Module given" (line 109): `given` "is for specs where rules operate on shared
  instances that exist once per module scope". The section does not say that a rule may
  create a `given` instance, and gives no way to state a `given` instance's field
  values.
- "Defaults" (line 1555): a `default` declaration creates a named entity instance, with
  field values, that "exist[s] unconditionally" and is available to all rules "without
  requiring creation by any rule".
- Validation rule 24 (line 1961): an unqualified instance reference in a rule must
  resolve to a `given` binding, a `let` binding, a trigger parameter or a default entity
  instance. Every rule that writes `queue.next_position` relies on the `given` binding
  today.
- "Preconditions (requires)" (line 818): when a `requires` fails on an external stimulus
  trigger, "The action is rejected; caller receives an error".
- "Config" (line 1463): a rule may reference `config.<name>` in a `requires`.
- Entity creation in an `ensures` uses `.created(...)` (line 900).
- Validation rule 30 (line 1973): a trigger a surface `provides` must be defined as an
  external stimulus trigger in a rule. Whether the pinned checker warns about an
  external stimulus trigger that no surface provides is **unverified**; step 4 finds
  out by running `just check-specs` and `just analyse-specs`.
- "Checking semantics" (line 1386): "Invariants are logical assertions over entity
  state, not runtime checks."

The Rust library:

- `crates/libpachinko/src/config.rs`: `Config` (lines 65 to 78) holds the three TTLs as
  `std::time::Duration` and the two limits as `u32`. `Config::check` (lines 95 to 103)
  returns `ConfigError::MaxConsecutiveExpiriesZero` and then
  `ConfigError::MaxPullBatchZero`, in that order, so it reports the first broken bound
  only. The type doc (lines 14 to 16) says the three TTL invariants "hold by type".
  Three `compile_fail,E0600` doctests (lines 45 to 62) show that negating a TTL does not
  compile; the first carries `// scenario: S005`.
- A `u32` cannot be negative either, so a negative limit is also excluded by type.
- `crates/libpachinko/src/error.rs`: `ConfigError` (lines 177 to 184) has those two
  variants, each message naming its invariant.
- `crates/libpachinko/src/queue.rs`: `Queue::new` (line 142) calls `config.check()?`
  before it builds anything, so a refused config makes no queue.
- `crates/libpachinko/src/queue/pockets.rs` line 46: the counter starts at
  `Position::default()`, and `Position` is a `u64` newtype that derives `Default`
  (`crates/libpachinko/src/records.rs`, lines 33 to 35). The first position is 0.
- `crates/libpachinko/tests/queue/identity.rs`: `a_zero_expiry_limit_is_refused` (S003,
  line 45) and `a_zero_pull_batch_is_refused` (S004, line 59).
- `crates/libpachinko/tests/queue/invariants.rs`: `check_config` (line 78) asserts the
  five config invariants on every step of the invariant walk.
- `crates/libpachinko/tests/queue/publish.rs`: the S011 test (line 24) asserts
  `next_position` 0 on a new queue, then pocket 0, record 1 and `next_position` 2.

The service:

- `crates/pachinko/src/config.rs`: `ServeConfig::parse` (line 96) sets the five values
  from `--lease-ttl-ms`, `--max-consecutive-expiries`, `--max-pull-batch`,
  `--superseded-ttl-ms` and `--dead-letter-ttl-ms` (lines 269 to 273), then calls
  `config.queue.check()?` (line 119). `millis` (lines 297 to 302) parses an unsigned
  integer, so `--lease-ttl-ms -1` is a `ServeError::Usage`, "takes milliseconds, not",
  and never reaches the library's check. `count` (lines 329 to 333) does the same for a
  negative limit.
- `crates/pachinko/src/error.rs` line 34: `ServeError::Config`, printed as
  `invalid configuration: <reason>`.
- `docs/reference/configuration.md`, "Service flags": a zero limit exits 1 with
  `pachinko: invalid configuration: <reason>`; a value that is not an unsigned integer
  exits 1 with `pachinko: usage: <reason>`.

The catalog, `docs/reference/testing.md`:

- S003 (line 197), "`max_consecutive_expiries = 0` is refused", and S004 (line 198),
  "`max_pull_batch = 0` is refused", both in `identity.rs`.
- S005 (line 199), "Negative TTLs are refused", owned by "the `compile_fail` doctest on
  `Config`".
- S011, S012, S014, S015, S016 and S022 (lines 205 to 216), all in
  `crates/libpachinko/tests/queue/publish.rs`.
- 93 scenarios, S001 to S093. `impl-choice` is not a catalog tag; the label lives in the
  double's tests and modules and in `double/SPEC-NOTES.md`.

The Python double. `double/` and `double/SPEC-NOTES.md` are on the branch
`build-python-double` and were not on `main` when this ticket was written.

- `double/pachinko_double/config.py`: `Config` (lines 16 to 34) is a frozen dataclass.
  Its `__post_init__` checks all five bounds and raises `ConfigRefused` naming every
  invariant the values break, in the specification's order. A TTL is an `int` of
  milliseconds, so a negative one is refused at run time.
- `double/pachinko_double/refusals.py` line 14: `ConfigRefused`, with its `invariants`
  tuple.
- `double/pachinko_double/queue.py` line 70: `next_position: int = 0`.
- `double/tests/test_config.py`: S003 (line 31), S004 (line 37), and S005 (line 45)
  under an `impl-choice (spec note N09)` comment. A fourth test shows a config that
  breaks three invariants names all three.
- `double/tests/test_publish.py`: the six rows, each under an `impl-choice` comment
  naming N04 or N16: S011 (line 63), S014 (line 94), S016 (line 114), S012 (line 166),
  S015 (line 181), S022 (line 242).
- `double/README.md` says the five config invariants are enforced when a `Config` is
  made, and that the positions it prints start at 0 (N04).
- `double/SPEC-NOTES.md`: findings 13 and 14 and notes N04, N09 and N16 name this
  ticket. Notes N31 and N38 list further absolute values that rest on N04: S048's and
  S088's arrival positions and S078 to S081's kept positions.

### The implementations differ today

| Point | Rust library and service | Python double |
| --- | --- | --- |
| A zero limit | Refused at `Queue::new`, as `ConfigError` | Refused at `Config(...)`, as `ConfigRefused` |
| A negative limit | Cannot be written (`u32`); the flag parser answers a usage error | Refused at `Config(...)` |
| A negative TTL | Cannot be written (`Duration`); the flag parser answers a usage error | Refused at `Config(...)` |
| Several broken bounds | The first only, expiry limit before pull batch | Every broken invariant, in the specification's order |
| When | Building the queue | Building the config, before any queue |
| First position | 0 | 0 |

### Risks (`CONVENTIONS.md` §13)

- **The specification moves first** (`AGENTS.md`, invariant 1). The start rule lands,
  and passes `just check-specs` and `just analyse-specs`, before any test or code.
- **A rule that creates what `given` declares.** The language reference does not say the
  two combine. A shape the checker accepts may still mean something other than the
  ruling. Step 3 puts the choice to the maintainer; step 4's review checks the meaning,
  not only the checker's exit status.
- **Every existing rule reads `queue`.** If the `given` block changes, all the rules
  that allot a position are touched. A changed rule body is a finding unless step 3
  authorised it.
- **A refusal that cannot be observed.** A refused start leaves no queue, so a test
  cannot compare a snapshot before and after. The double's three config tests are
  already the stated exception to that practice (N09); the Rust's tests assert the
  error and that the boundary value builds.
- **A first position of 0 pinned by accident.** If the maintainer rules "unspecified",
  a test that still asserts an absolute position is wrong, in the Rust as in the
  double.
- **Counts move.** The catalog and the handbook state the number of scenarios,
  invariants and config values. Each is re-counted from the module, not edited
  by hand from this ticket's figures.
- **A public surface may change.** If `ConfigError` gains or renames a variant, or
  `Config::check` changes what it reports, `just semver` reports it, and each changed
  public item keeps invariant 3 and has a doctest.
- **The small denominator.** Each new branch has its test, or coverage falls below 90%.
- **lychee and typos over `tickets/`.** The hand-back notes write paths and test names
  as code spans, never as links.

## Goal

The specification states the start of a deployment as a rule: a trigger whose `requires`
are the config bounds and whose `ensures` creates the Queue with the first position the
maintainer ruled. A config outside a bound is refused under the name T43 decided, and no
Queue is made. The Rust library, the service, the double and the catalog agree with the
rule. The six rows S011, S012, S014, S015, S016 and S022, and S005, are either the
specification's or reworded, as the maintainer's answers at step 3 decide, and no note
in `double/SPEC-NOTES.md` still calls N04, N09 or N16 open.

## Non-goals

- No write-ahead log, no recovery and no restart of a deployment. The maintainer's note
  on them is recorded where step 3 decides; nothing is built for it.
- No new bound. "A positive floor is the maintainer's to set" stays as it is; the bounds
  are the five the invariants state.
- No change to a default value.
- No bound on a rank (T44) and no change to the pocket-order model (T40).
- No naming of any refusal other than the start rule's, and no change to the order other
  refusals are checked in (T43).
- No change to the service's own flags (`--listen`, the request-shape limits, the
  timeouts), which are not specification values.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | shared, own lines | The start rule; the `given` block and `entity Queue` as step 3 decides; the comments on the five config invariants; the header's Includes line if the rule adds a term |
| `crates/libpachinko/src/config.rs`, `error.rs`, `queue.rs`, `queue/pockets.rs`, `records.rs`, `lib.rs` | shared, own lines | The refusal under its decided name and check order; the first position; the doc comments that say the TTL invariants "hold by type", reworded to what step 3 decides |
| `crates/libpachinko/tests/queue/identity.rs`, `publish.rs`, `invariants.rs`, `fixture.rs`, `walk/model.rs` | shared, own lines | S003 and S004 against the rule; the six rows as step 3 decides; the new obligations' tags |
| `crates/pachinko/src/config.rs`, `error.rs` | shared, own lines | The flags against the rule, as step 3 decides for a negative value |
| `crates/pachinko/tests/*.rs` | shared, own lines | A wire or binary test for the refusal, if step 3 asks for one |
| `docs/reference/testing.md`, `docs/reference/configuration.md`, `docs/reference/api.md` | shared, own lines | The rows for S003, S004, S005 and the six position rows; any new row; the counts; the flag text |
| `docs/explanation/specifications.md`, `docs/how-to/work-with-the-specs.md`, `docs/project/terminology.md` | shared, own lines | Only where a count or a term is made false |
| `CHANGELOG.md`, `crates/libpachinko/README.md` | shared, own lines | The Unreleased entries; any signature shown |
| `double/pachinko_double/config.py`, `refusals.py`, `queue.py`, `exceptions.py`, `double/tests/test_config.py`, `test_publish.py`, `double/README.md` | shared, own lines, if the base holds it | The refusal's name and report; the first position; the `impl-choice` comments on S005 and the six rows |
| `double/SPEC-NOTES.md`, `double/diagrams/*` | shared, own lines, if the base holds it | Findings 13 (first position) and 14 marked applied; N04, N09 and N16; N31 and N38 where they cite N04; a figure or alt text only if made false |
| `tickets/README.md` | shared, own lines | T47's Status cell |
| `tickets/T47-start-rule-and-config-bounds.md` | ticket | `status:` and hand-back notes |

## Steps

0. **The worktree and the baseline.** Create the branch and its worktree from `main`
   after T40 and T43 have merged (`CONVENTIONS.md` §11). Run `just initialize` (network,
   authorised by this step). Run `just check` before any edit and quote its last line
   as the baseline.
1. **Read what the dependencies decided.** Read, and quote with their paths:
   - T43's merged decision record: the name of the start rule's refusal, how Allium
     states a named refusal, and the order refusals are checked in. **Stop** if the
     record is absent or does not name this refusal, and ask the maintainer.
   - T40's merged decision record and the follow-up tickets it created: what a position
     is, and where the pocket order lives. Say which follow-ups have merged.
   - Whether T39, T42's follow-ups and T45 have merged, and what each changed in the
     `config` block, in `entity Queue` and in the surfaces.

   If `main` does not hold `double/`, say so; step 3 asks whether the double moves here
   or in a follow-up.
2. **The plan.** Use the `plan-change` skill. Read the whole module and
   `just plan-spec pachinko`'s output, and re-read everything "What exists today"
   names, correcting each line number and count. Read the language reference sections
   that Context cites. Then draft the start rule, in the forms the language allows, and
   check each draft. `just check-specs`, `just analyse-specs` and `just plan-spec` read
   only the modules under `docs/specs/`, so a copy under `ai_tmp/` is never checked.
   For each draft in turn: apply it to `docs/specs/pachinko.allium` uncommitted, run
   the three recipes and keep their output, run `mkdir -p ai_tmp/t47`, save the draft with
   `git diff -- docs/specs/pachinko.allium >| ai_tmp/t47/<form>.patch`, then reverse it
   with `git apply -R ai_tmp/t47/<form>.patch` and show `git status --short` empty
   before the next. This step authorises that reversal of the step's own trial edit
   and nothing else. The forms to try, as a starting list and not a ruling:
   - The `given` block kept, and a rule whose `ensures` is `Queue.created(...)`.
   - The `given` block removed, and every rule that reads `queue` binding it another
     way that validation rule 24 accepts.
   - A `default` instance for the Queue. It states a first value, but it exists
     unconditionally, so it does not by itself express "the rule creates the Queue".

   For each form write down: whether the checker and the analyser accept it and with
   what warnings; whether a second start is possible and what refuses it; how the first
   position is written, both as 0 and as unspecified; which existing rules change; and
   the obligations `just plan-spec` adds.

   **Adversarial review** (`CONVENTIONS.md` §14): the drafts and the comparison, before
   they go to the maintainer. Does each draft mean what the ruling says? Is anything
   the language reference does not state presented as fact? Is an option missing?
3. **Authorisation required: the open points.** Put each to the maintainer, quoting
   this ticket's Context and step 2's comparison, and assume none:
   1. **The first position.** Unspecified, or 0, read on the position model T40
      decided. Say what each means for the six rows: with 0 they become the
      specification's and lose `impl-choice`; with "unspecified" ask whether they stay
      labelled or are rewritten to assert only order and steps, in the Rust as in the
      double. State the maintainer's note on a write-ahead log and recovery, and ask
      where it is recorded (a comment on the rule, the Excludes line, or nowhere).
   2. **S005 and compile time.** The rule refuses a negative TTL at run time. The Rust
      cannot represent one: `Duration` and `u32` exclude negatives by type, and the
      service's flag parser answers a usage error, not the config refusal. Ask whether
      that satisfies the rule. The options to lay out: the type-level exclusion
      satisfies it, and S005 stays a `compile_fail` doctest with its description saying
      so; or the rule's refusal must be observable under its decided name, and then say
      where (the service's flags only, or the library too) and what S005's owning test
      becomes. The same question covers a negative limit.
   3. **The shape of the rule and the fate of `given`**, if step 2 found that the
      language forces a choice: the forms that pass, and what each costs.
   4. **Who starts a deployment.** Whether the trigger is provided by a surface, and
      facing whom, or stands with no surface. Quote what the checker said about a
      trigger no surface provides.
   5. **The five invariants.** The ruling makes the bounds the rule's `requires`. Ask
      whether the five `invariant` blocks stay, restating what the rule guarantees since
      no rule changes a config value, or go. If they stay, their "The model is defined
      for" comments are reworded to name the rule. The count of 35 depends on the
      answer.
   6. **Several broken bounds.** One refusal naming every broken bound, as the double
      does, or the first in a fixed order, as the Rust does, if T43's decision does not
      already settle it.
   7. **The double's base**, if step 1 found `double/` absent from `main`.

   Then stop until the maintainer answers, and quote the answers in the hand-back,
   dated. An answer that names something outside these points is a new design: record
   it and stop again.
4. **The specification.** Use the `spec-change` skill. Land the rule as step 3 decided,
   with the refusal's name from step 1; reword the comments on the five config
   invariants; change `given` and `entity Queue` only as authorised. Run
   `just check-specs` and `just analyse-specs`, then `just plan-spec pachinko` for the
   new obligations.

   **Adversarial review** (`CONVENTIONS.md` §14): the specification diff against the
   two rulings and step 3's answers. Does every bound appear in a `requires`, with the
   boundary values 1 and 0 admitted? Can any rule run before the Queue exists? Did a
   rule body change that step 3 did not authorise? Does the module still say nothing
   more about recovery than was ruled?
5. **The tests, red first.** Derive each obligation test from the changed clauses and
   show each failing before the code changes; quote the failures. A test that cannot be
   red because the code already behaves so is named as such, with the reason. Give any
   new scenario the catalog's next free id, read from the catalog at this step.
6. **The code.** Use the `rust-change` skill. The library first, then the service, in
   one commit or in commits that each leave `just check` green. Run `just fmt-check`,
   `just clippy`, `just test`, `just coverage`, `just doc`, `just effects` and
   `just semver` as the change needs them.
7. **The double**, if the base holds it: the refusal's name and report, the first
   position, and the `impl-choice` comments on S005 and the six rows, each removed,
   kept or reworded as step 3 decided. Find every one with `rg -n 'impl-choice' double`
   and change only those that cite N04, N09 or N16. Run `just python-double`.
8. **The notes and the handbook.** In `double/SPEC-NOTES.md`, mark finding 14 and the
   first-position part of finding 13 as applied, in the form the answered findings
   already use; update N04, N09 and N16, the rulings table, and N31 and N38 where they
   rest on N04. Use the `review-docs` skill for every page in Files touched. Run
   `just check-docs`.
9. **Close.** Set `status: done` here and in `tickets/README.md`, and write the
   hand-back notes.

   **Adversarial review** (`CONVENTIONS.md` §14): the whole diff, both lenses. Then run
   `just check`.

   **Authorisation required:** pushing the branch and opening the pull request are each
   asked for separately.

## Acceptance criteria

- Step 1 quotes T43's and T40's merged decisions with their paths, and nothing was
  named or positioned before they were read.
- The maintainer's answers at step 3 are quoted in the hand-back with their date, and
  nothing before them chose an option.
- `docs/specs/pachinko.allium` has one rule for the start of a deployment. Each config
  bound in force is a `requires` of it, it creates the Queue, and it gives
  `next_position` what step 3 decided. `just check-specs` and `just analyse-specs`
  pass, and the analyser's warnings are quoted.
- A config outside a bound makes no queue in the Rust library, in the service and in
  the double, and is refused under the name T43 decided, as far as step 3's answer on
  compile time requires. The boundary values, 1 for each limit and 0 for each TTL, are
  admitted in all three.
- S003, S004 and S005 each have an owning test that agrees with the rule and with step
  3's answer, and S005's description in the catalog says whether the refusal is by type
  or at run time.
- The six rows S011, S012, S014, S015, S016 and S022 are as step 3 decided, the same
  way in the Rust tests and in the double's, and no `impl-choice` comment still cites
  N04, N09 or N16 for a point this ticket settled.
- `double/SPEC-NOTES.md` marks finding 14 and finding 13's first position as applied,
  and N04, N09 and N16 say what the specification now states.
- Every `// obligation:` id in `crates/` is in `just plan-spec pachinko`'s output or
  names a `@guarantee` of the module, which the planner gives no id
  (`docs/reference/testing.md`).
- The catalog's counts of scenarios, invariants and config values match the
  module, and the scenario ids run contiguously, each tagged once.
- Every new or changed public item keeps invariant 3 and has a doctest; `just coverage`
  reports at or above 90%; `just effects` passes; the `just semver` report is quoted.
- No write-ahead log, recovery or restart behaviour was added.
- Each adversarial review in Steps ran, and every valid finding is fixed or handed back.
- `just check` ends with `All checks passed and the worktree is unchanged.` and
  `git status --short` prints nothing.

## Verification

Each command's exit status is quoted (`CONVENTIONS.md` §11, "Checks fail closed"). A
search that must find nothing passes only on `rg`'s status 1, never on an error, and
follows a positive control over the same files. `<StartRule>` is the rule's name as
landed; replace it before running. At step 0, before any edit, the `impl-choice` search
is run once over `double` if the base holds it and must match: that shows it can.

```sh
set -o pipefail
just check-specs; echo "exit $?"
just analyse-specs; echo "exit $?"
just plan-spec pachinko; echo "exit $?"
# the start rule exists, once (prints 1, exit 0)
rg -c '^rule <StartRule> ' docs/specs/pachinko.allium; echo "exit $?"
# positive control: the five bounds are still found in the module (exit 0)
rg -n 'max_consecutive_expiries >= 1|max_pull_batch >= 1|_ttl >= 0\.seconds' \
  docs/specs/pachinko.allium; echo "exit $?"
# positive control: the label is still found in the double (exit 0); skip both
# searches if the base does not hold double/
rg -c 'impl-choice' double/tests double/pachinko_double; echo "exit $?"
# no impl-choice comment cites a note this ticket settled: rg must exit 1.
# Drop a note from the pattern only if step 3 ruled that its rows stay labelled.
rg -n 'impl-choice \(spec note N(04|09|16)\)' double/tests double/pachinko_double
[ "$?" -eq 1 ]; echo "exit $?"
just python-double; echo "exit $?"
just test; echo "exit $?"
just coverage; echo "exit $?"
just effects; echo "exit $?"
just semver; echo "exit $?"
just check-docs; echo "exit $?"
just check; echo "exit $?"
git status --short; echo "exit $?"
```

## Open points

- The seven points of step 3 are the maintainer's to settle: the first position and
  where the write-ahead-log note is recorded; whether the Rust's compile-time refusal
  satisfies the rule (S005); the shape of the rule and the fate of `given`; who starts
  a deployment; whether the five invariants stay; one refusal or the first of several;
  and the double's base.
- The refusal's name and how Allium states it are T43's. This ticket stops at step 1
  without them.
- Whether this ticket waits for a T40 follow-up that changes `Queue` or positions is
  asked at step 1 if one is unmerged.
- Whether the pinned checker warns about an external stimulus trigger that no surface
  provides is unverified; step 2 runs the checker to find out.
- No ticket for a write-ahead log or recovery exists. When one is written, it re-reads
  the first position this ticket settled.

## Hand-back notes

None yet.

### Step 0: the baseline

### Step 1: what T43, T40 and the other tickets decided

### Step 2: the drafts and the comparison

### Step 3: the maintainer's answers

### Steps 4 to 8: the specification, the tests, the code, the double, the notes

### Adversarial reviews

### Verification
