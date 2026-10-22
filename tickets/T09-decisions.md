---
id: T09
title: "Decision records 0001 to 0014"
status: open
depends_on: [T00]
parallel_with: [T01, T02, T03, T04, T05, T06, T07, T08, T10]
branch: ticket/t09-decisions
estimated_size: M
---

# T09: Decision records 0001 to 0014

## Context

T00 registered fourteen decision records and the decision index, each a stub.
`CONVENTIONS.md` §8 gives each record's file name, title, slug and source: eleven
carried from L, adapted (0003 most, widened from one effect to all), and three new (0001,
0008, 0014).
A carried record opens "Carried from libpawdoku's decision NNNN at `ccdd001`" and keeps
L's slug. Wave-2 tickets write 0015 to 0018 themselves.

L `tickets/T09-decisions.md` is the procedure and L `docs/decisions/README.md` the index
format. The maintainer's decisions behind the new records are `CONVENTIONS.md` §1,
decisions 1 to 8.

## Goal

Fifteen final files: the index and fourteen records, each with L's sections (Context,
Decision, Consequences, What would reopen this, Related pages), each true of this
repository, and none restating another.

## Non-goals

- No record beyond 0014; no manifest change (frozen).
- No decision the maintainer has not taken. Where a record would need one (for example
  0014's eventual runtime), it states the question and the trigger, not an answer.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/decisions/README.md` | final | the index, L's format |
| `docs/decisions/0001-library-and-service.md` | final | new: `CONVENTIONS.md` §1 decisions 1, 4 |
| `docs/decisions/0002-specs-are-the-source-of-truth.md` | final | L 0002 |
| `docs/decisions/0003-effects-behind-traits.md` | final | L 0003, widened: every effect; the service implements them |
| `docs/decisions/0004-hook-runner-and-checkers.md` | final | L 0004 |
| `docs/decisions/0005-project-managed-allium-cli.md` | final | L 0005 |
| `docs/decisions/0006-apache-2-0.md` | final | L 0006, both crates, `publish = false` |
| `docs/decisions/0007-dependency-policy.md` | final | L 0007 |
| `docs/decisions/0008-std-library.md` | final | new: why std, and gate 8 in place of `no_std` |
| `docs/decisions/0009-rust-quality-gate.md` | final | L 0009, without the wasm and no_std parts |
| `docs/decisions/0010-skill-reference-material.md` | final | L 0010 |
| `docs/decisions/0011-tool-manager.md` | final | L 0011 |
| `docs/decisions/0012-metrics-gate.md` | final | L 0013, thresholds kept, layering rules waiting for modules |
| `docs/decisions/0013-api-reference-on-github-pages.md` | final | L 0016, two crates |
| `docs/decisions/0014-service-runtime-deferred.md` | final | new: `CONVENTIONS.md` §1 decision 4 |
| `tickets/T09-decisions.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. **The carried records.** For each, read L's record at `ccdd001` (L
   `docs/decisions/0002-specs-are-the-source-of-truth.md` and its siblings; the numbers
   map as `CONVENTIONS.md` §8 shows). Keep its argument; remove what is sudoku, no_std or
   wasm; rewrite what names L's crates or URLs. Where L's Context cites a pawdoku
   decision, keep the chain ("carried from libpawdoku's 0004, itself carried from
   Pawdoku's 0004").
3. **0001.** Why a library and a service in one workspace: the domain lives where it can
   be tested without effects; the service is thin; one lockfile and one gate. What would
   reopen it: a second consumer of the library that needs its own release cadence.
4. **0003 and 0008.** Divide the argument as `CONVENTIONS.md` §8 says. 0003: the
   pattern, every effect, fakes beside the trait, the service's implementations. 0008:
   L's 0008 chose `no_std` to make effects unnameable; this repository is std
   (`CONVENTIONS.md` §1 decision 2), so it names the effects and bans them in libpachinko
   instead, through gate 8, with a fixture that proves each ban fires. Name the
   mechanism as T10's choice, for T11 to fill in.
5. **0012.** L's 0013 with its thresholds; its module-boundary rules are not carried
   because there are no modules; they return with the first spec that names two.
6. **0014.** No async runtime, transport or HTTP stack is chosen. The trigger is the
   first specification that names a surface the service exposes; then a spike compares
   options and a record supersedes this one. Until then the service is `run()` over a
   writer.
7. **The index.** L's format, fourteen rows, plus a line that 0015 to 0018 are written by
   T13 to T17.
8. `just check-docs`, `just check`, then commit.

## Acceptance criteria

- `just check-docs` is green, and every record's frontmatter equals its manifest entry.
- Every carried record opens with the carried-from sentence; every new record has none.
- `rg -n -i 'sudoku|wasm|no_std' docs/decisions` prints only lines in 0008 that explain
  why `no_std` was not chosen.
- `just check` is green.

## Verification

```sh
just check-docs
rg -n -i 'sudoku|wasm|no_std' docs/decisions
rg --files-without-match 'Carried from libpawdoku' docs/decisions/000[2-79]-*.md docs/decisions/001[0-3]-*.md
just check
```

The second `rg` must print nothing: every carried record has the sentence.

## Hand-back notes

To be written by the executing agent: for each carried record, what was removed and why;
sentences that wait on T10's mechanism (for T11).

## Open points

None.
