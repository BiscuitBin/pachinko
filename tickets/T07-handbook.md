---
id: T07
title: "Handbook: every page, and decision records 0001 to 0014"
status: open
depends_on: [T00]
parallel_with: [T01, T02, T05, T06, T10]
branch: ticket/t07-handbook
estimated_size: XL
---

# T07: Handbook: every page, and decision records 0001 to 0014

## Context

T00 registered every page and record in `docs/manifest.yml` and left a stub for each.
This lane writes the handbook pages `CONVENTIONS.md` §6 assigns to T07, each adapted from
L's page of the same path, and the decision index with records 0001 to 0014
(`CONVENTIONS.md` §8). T06 owns the two specification pages; wave-2 tickets write records
0015 to 0018 themselves.

The work is in three parts, in order, each closed by its own review:

- **Part 1**, steps 1 to 5: the eight project, tutorial and how-to pages.
- **Part 2**, steps 6 to 10: the four explanation pages, the seven reference pages and
  the two operations pages. The architecture page carries the argument this repository
  is built on: a library that performs no effect, and a service that supplies them.
- **Part 3**, steps 11 to 17: the decision index and records 0001 to 0014.

L `docs/reference/documentation-contract.md` is the rulebook (frontmatter equals the
manifest entry, one owning page per topic, 40 words minimum, links that resolve with
exact case). L `tickets/T07-handbook-a.md`, L `tickets/T08-handbook-b.md` and L
`tickets/T09-decisions.md` are the procedures for the three parts, and L
`docs/decisions/README.md` is the index format.

Several facts parts 2 and 3 state are settled by other wave-1 lanes running at the same
time (T02's lint changes, T05's skills, T10's effects mechanism). Write from
`CONVENTIONS.md` and L; where a lane's outcome could change a sentence, write the
sentence to the design and list it in the hand-back notes for T11 to reconcile.

`CONVENTIONS.md` §8 gives each record's file name, title, slug and source: eleven carried
from L, adapted (0003 most, widened from one effect to all), and three new (0001, 0008,
0014). A carried record opens "Carried from libpawdoku's decision NNNN at `ccdd001`" and
keeps L's slug. The maintainer's decisions behind the new records are `CONVENTIONS.md`
§1, decisions 1 to 8.

## Goal

Twenty-one final pages and fifteen final decision files (the index and fourteen records)
that describe this repository as it is after wave 1: a two-crate std workspace whose
domain is undefined, with the effects boundary, the gate and the tools L has. The effects
boundary is explained once, in `architecture.md`, and referred to everywhere else. Each
record has L's sections (Context, Decision, Consequences, What would reopen this, Related
pages), is true of this repository, and restates no other. `just check-docs` is green.

## Non-goals

- No page or record outside this lane's twenty-one pages, the index and records 0001 to
  0014; no manifest change and no new page (frozen).
- No domain words. `docs/project/terminology.md` seeds the glossary with the foundation's
  own vocabulary and nothing the specifications will define.
- No runtime, transport or HTTP stack described as chosen (decision 0014).
- No description of a wave-2 tool as if it existed. A page may say what wave 2 adds,
  in a sentence, naming the ticket.
- No decision the maintainer has not taken. Where a record would need one (for example
  0014's eventual runtime), it states the question and the trigger, not an answer.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/project/purpose-and-scope.md` | final | `CONVENTIONS.md` §6 |
| `docs/project/repository-map.md` | final | the tree of `CONVENTIONS.md` §3 |
| `docs/project/terminology.md` | final | the glossary seed |
| `docs/tutorials/first-change.md` | final | the domain-neutral exercise |
| `docs/how-to/develop-locally.md` | final | L's, two crates |
| `docs/how-to/maintain-dependencies.md` | final | L's; nothing updates the pins until T17 |
| `docs/how-to/test-and-debug.md` | final | L's, per crate, the binary test |
| `docs/how-to/deploy-to-github-pages.md` | final | both crates' rustdoc on this repository's Pages |
| `docs/explanation/architecture.md` | final | the split, the boundary, what is deferred, what wave 2 adds |
| `docs/explanation/layering.md` | final | crate direction; module direction later |
| `docs/explanation/quality-philosophy.md` | final | L's argument, this gate's examples |
| `docs/explanation/security-model.md` | final | supply chain, unsafe, cargo-deny, the boundary |
| `docs/reference/commands.md` | final | every recipe of the `Justfile` |
| `docs/reference/configuration.md` | final | every configuration file |
| `docs/reference/testing.md` | final | nextest, doctests, the binary test, coverage, fakes, snapshots |
| `docs/reference/quality-gates.md` | final | the nineteen gates |
| `docs/reference/documentation-contract.md` | final | L's rules verbatim; "Links that leave the repository" names only the outward pages that exist here: decision 0005, `api.md`, `deploy-to-github-pages.md` |
| `docs/reference/agent-contract.md` | final | fourteen skills; no `skills-lock.json` (`CONVENTIONS.md` §7) |
| `docs/reference/api.md` | final | rustdoc for both crates |
| `docs/operations/maintenance.md` | final | L's routine |
| `docs/operations/troubleshooting.md` | final | L's, minus wasm |
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
| `docs/decisions/0010-skill-reference-material.md` | final | L 0010, corrected: no `skills-lock.json`, and the vendored files are the only record of upstream (`CONVENTIONS.md` §7) |
| `docs/decisions/0011-tool-manager.md` | final | L 0011 |
| `docs/decisions/0012-metrics-gate.md` | final | L 0013, thresholds kept, layering rules waiting for modules |
| `docs/decisions/0013-api-reference-on-github-pages.md` | final | L 0016, two crates |
| `docs/decisions/0014-service-runtime-deferred.md` | final | new: `CONVENTIONS.md` §1 decision 4 |
| `tickets/T07-handbook.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. **Part 1: project, tutorial and how-to pages.** For each part-1 page, read L's page of the same path at `ccdd001` and
   `CONVENTIONS.md` §6's adaptation note. Keep L's structure and every sentence that is
   still true; rewrite what names sudoku, pawdoku, wasm, `no_std`, L's modules or L's
   URLs. Where L says `just initialize` normalises formatting (`develop-locally.md`,
   `commands.md`, `first-change.md`), drop that clause (`CONVENTIONS.md` §5). Parts 2
   and 3 apply the same rule to their pages and records.
3. **`terminology.md`.** One entry each for: the repository, the library
   (`libpachinko`), the service (`pachinko`), an effect, an effect trait, a fake, a
   specification, a module, a gate, a recipe. Each entry says which word to use and
   which to avoid, in L's style. State that the domain's words arrive with the
   specifications and are added here by the ticket that adds them.
4. **`first-change.md`.** An exercise a contributor can follow again on any later
   `main`: change the line `run()` writes, watch `just test` fail in two places (the unit
   test and the binary test), update both, then `just check`. Follow it yourself in a
   scratch worktree, start to finish, and quote the commands and failures; a tutorial
   no one has run is not done.
5. **`deploy-to-github-pages.md`.** The `pages.yml` T01 writes and the `site` recipe;
   the URL is the one `gh api repos/BiscuitBin/pachinko/pages --jq .html_url` prints after
   T01, written here as `https://biscuitbin.github.io/pachinko/` with a note to confirm it.
   Then `just check-docs`, `just check`, the part-1 review, `just check` again if fixes
   landed, then commit part 1.

   **Adversarial review** (`CONVENTIONS.md` §14): part 1's diff and step 4's
   transcript. Ask: which sentence kept from L is false in this repository; whether a
   contributor on a later `main` can replay the tutorial exactly as written.

6. **Part 2, `architecture.md`.** Two crates and why (decision 0001). libpachinko holds the
   domain once specified and performs no effect: the clock, randomness, the filesystem,
   the network, the environment and threads reach it through traits it defines, with
   fakes beside them for tests (decision 0003). pachinko implements those traits and is
   the only place effects happen. Gate 8 enforces it (decision 0008); name the bans as
   `CONVENTIONS.md` §9 lists them, and the mechanism as "T10's", to be named by T11. What
   is deferred: the runtime, transport and HTTP stack, until a specification names a
   surface the service exposes (decision 0014). What wave 2 adds, one sentence each,
   naming the ticket and not described as existing: the container image (T13) and the
   release process (T14). Neither is deferred (`CONVENTIONS.md` §1 decision 8); each
   ticket makes its sentence present tense when it lands.
7. **`layering.md`.** Cargo enforces `pachinko -> libpachinko` and never the reverse.
   Inside libpachinko, module direction will mirror the specification import graph, and
   rustqual's pattern rules (decision 0012) will hold it once there are modules.
8. **`quality-philosophy.md`.** L's argument verbatim where it holds; L's sudoku examples
   replaced with this gate's: the effects probe, the binary test, the coverage floor on a
   small denominator.
9. **The reference pages.** `commands.md` lists every recipe in the `Justfile` with its
   comment's meaning (`just --list` is the check); `quality-gates.md` the nineteen gates
   of `CONVENTIONS.md` §4 in order; `configuration.md` every configuration file in
   `CONVENTIONS.md` §3; `testing.md` per L, plus the binary test and the insta rule from
   L `tickets/T29-snapshot-harness.md` (a snapshot shows a value to a reviewer; it never
   proves a clause) with the note that `insta` arrives with the first snapshot.
10. **The operations pages.** L's, with this repository's names; troubleshooting loses
    the wasm entries. Then `just check-docs`, `just check`, the part-2 review,
    `just check` again if fixes landed, then commit part 2.

    **Adversarial review** (`CONVENTIONS.md` §14): part 2's diff, since part 1's commit.
    Ask: which fact contradicts `CONVENTIONS.md` or the `Justfile`; whether
    `quality-gates.md` lists the gates in `CONVENTIONS.md` §4's order; which sentence
    waits on T10's mechanism and is not yet listed for T11; which sentence calls one of
    `CONVENTIONS.md` §1 decision 8's extras deferred, or describes one as existing.

11. **Part 3, the carried records.** For each, read L's record at `ccdd001` (L
    `docs/decisions/0002-specs-are-the-source-of-truth.md` and its siblings; the numbers
    map as `CONVENTIONS.md` §8 shows). Keep its argument; remove what is sudoku, no_std
    or wasm; rewrite what names L's crates or URLs. Where L's Context cites a pawdoku
    decision, keep the chain ("carried from libpawdoku's 0004, itself carried from
    Pawdoku's 0004").
12. **0001.** Why a library and a service in one workspace: the domain lives where it can
    be tested without effects; the service is thin; one lockfile and one gate. What would
    reopen it: a second consumer of the library that needs its own release cadence.
13. **0003 and 0008.** Divide the argument as `CONVENTIONS.md` §8 says. 0003: the
    pattern, every effect, fakes beside the trait, the service's implementations. 0008:
    L's 0008 chose `no_std` to make effects unnameable; this repository is std
    (`CONVENTIONS.md` §1 decision 2), so it names the effects and bans them in
    libpachinko instead, through gate 8, with a fixture that proves each ban fires. Name
    the mechanism as T10's choice, for T11 to fill in.
14. **0012.** L's 0013 with its thresholds; its module-boundary rules are not carried
    because there are no modules; they return with the first spec that names two.
15. **0014.** No async runtime, transport or HTTP stack is chosen. The trigger is the
    first specification that names a surface the service exposes; then a spike compares
    options and a record supersedes this one. Until then the service is `run()` over a
    writer.
16. **The index.** L's format, fourteen rows, plus a line that 0015 to 0018 are written by
    T13 to T17.
17. `just check-docs`, `just check`, the final adversarial review, `just check` again if
    fixes landed, then commit.

    **Adversarial review** (`CONVENTIONS.md` §14): part 3's diff, since part 2's commit.
    Ask: which carried argument still leans on `no_std` or on sudoku; whether 0003 and
    0008 divide the argument as `CONVENTIONS.md` §8 says; whether every slug and title
    matches §8's table and the manifest.

## Acceptance criteria

- `just check-docs` is green, and every record's frontmatter equals its manifest entry.
- `rg -n -i 'pawdoku|sudoku|wasm|no_std|steven-cutting/libpawdoku' docs/project docs/tutorials docs/how-to/develop-locally.md docs/how-to/maintain-dependencies.md docs/how-to/test-and-debug.md docs/how-to/deploy-to-github-pages.md`
  prints nothing, or only a sentence naming L as the source.
- The tutorial was followed end to end, and the hand-back notes quote it.
- Every recipe `just --list` prints appears in `docs/reference/commands.md`, and every
  gate in `CONVENTIONS.md` §4 in `docs/reference/quality-gates.md`, in order.
- `rg -n -i 'pawdoku|sudoku|wasm|no_std' docs/explanation docs/reference docs/operations`
  prints nothing outside the two pages T06 owns, or only a sentence naming L as the
  source.
- Every carried record opens with the carried-from sentence; every new record has none.
- `rg -n -i 'sudoku|wasm|no_std' docs/decisions` prints only lines in 0008 that explain
  why `no_std` was not chosen.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just check-docs
just --list
rg -n -i 'pawdoku|sudoku|wasm|no_std' docs/project docs/tutorials docs/how-to
rg -n -i 'pawdoku|sudoku|wasm|no_std' docs/explanation docs/reference docs/operations
rg -n -i 'sudoku|wasm|no_std' docs/decisions
rg --files-without-match 'Carried from libpawdoku' docs/decisions/000[2-79]-*.md docs/decisions/001[0-3]-*.md
just check
```

The last `rg` must print nothing and exit 1, after the same globs passed to `ls` list
every carried record (the positive control of `CONVENTIONS.md` §11): every carried
record has the sentence.

## Hand-back notes

To be written by the executing agent: the tutorial transcript; each sentence kept from L
that needed a fact checked and how it was checked; every sentence written to the design
that another lane's outcome could change, including those that wait on T10's mechanism
(for T11); for each carried record, what was removed and why; the review record for each
checkpoint (`CONVENTIONS.md` §14); and anything handed back.

## Open points

None.
