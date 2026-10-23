---
id: T07
title: "Handbook: every page, and decision records 0001 to 0014"
status: done
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

### Baseline

At `bf51e0d` (untouched worktree, after `just initialize`, which the secondary worktree
needed because it had no pixi environment; `CONVENTIONS.md` §11 authorises it for every
new worktree), `just check` ended with
`All checks passed and the worktree is unchanged.` (exit 0).

The work is on the worktree's branch, `ticket/07-handbook`, not the ticket's
`ticket/t07-handbook`; T06's worktree uses the same unprefixed form. Rename it before
pushing if the `t` form matters.

### Commits

- `d631186` part 1: the project, tutorial and how-to pages.
- `97edd83` part 2: the explanation, reference and operations pages.
- `7896faa` part 3: the decision index and records 0001 to 0014.
- `012b884` the index's sentence on gates 7 and 8 (a contradiction with 0012 the
  hand-back consolidation found).

### The tutorial, followed end to end

Run in a fresh detached scratch worktree at `bf51e0d` (the crates part 1 does not touch),
every step of `docs/tutorials/first-change.md` in order, then the worktree removed. Full
transcript: `ai_tmp/T07/tutorial-replay.txt`.

```text
== scratch worktree at bf51e0d
$ git status --short                      exit 0 (empty)
$ just initialize                         exit 0
  Secondary worktree: skipping install-hooks.
$ just check                              exit 0
  All checks passed and the worktree is unchanged.
$ just doc                                exit 0
$ test -f target/doc/pachinko/index.html  exit 0
$ sed -i '' 's/"pachinko {}"/"pachinko version {}"/' crates/pachinko/src/lib.rs   exit 0
$ just test                               exit 100
        FAIL [   0.008s] (2/3) pachinko tests::run_writes_the_name_and_the_library_version
  Cancelling due to test failure: 1 test still running
        FAIL [   0.183s] (3/3) pachinko::binary the_binary_prints_its_line_and_exits_zero
     Summary [   0.183s] 3 tests run: 1 passed, 2 failed, 0 skipped
$ sed -i '' 's/b"pachinko 0.1.0\n"/b"pachinko version 0.1.0\n"/' crates/pachinko/src/lib.rs crates/pachinko/tests/binary.rs   exit 0
$ just test                               exit 0
     Summary [   0.190s] 3 tests run: 3 passed, 0 skipped
$ just check                              exit 0
  All checks passed and the worktree is unchanged.
$ git checkout -- crates/                 exit 0
$ git status --short                      exit 0 (empty)
```

The code review then narrowed step 7 to the two files the exercise changes. Re-run at
`a0b44b0` after making the step 4 and 5 edits (`ai_tmp/T07/tutorial-step7.txt`):

```text
$ git status --short
 M crates/pachinko/src/lib.rs
 M crates/pachinko/tests/binary.rs
$ git restore crates/pachinko/src/lib.rs crates/pachinko/tests/binary.rs   exit 0
$ git status --short                      exit 0 (empty)
```

The ticket predicted two failures and that is what happened, but `.config/nextest.toml`
sets `fail-fast = true`, so the second failure is reported only because both tests had
already started; the page says so and gives the `--no-fail-fast` command. That command,
`pixi run --frozen cargo nextest run --workspace --all-features --locked --no-fail-fast`,
was run at `bf51e0d` with part 1 uncommitted: `3 tests run: 3 passed`, exit 0. The replay
started from a warm cache in a secondary worktree, so the primary checkout's
`install-hooks` step was not exercised.

### Adversarial review, checkpoint 1 (`ai_tmp/review/T07-1/`)

Reviewers: Codex CLI 0.163.0, `codex exec -m gpt-6-astra -s read-only`, over part 1's
uncommitted diff at `bf51e0d` and the tutorial transcript. The conformance run finished.
The breakage run was killed with its wrapper the first time, and on the rerun Codex
answered `Selected model is at capacity`; the breakage lens was then run by a
fresh-context Claude Opus 5.5 subagent with the same brief (`CONVENTIONS.md` §14 allows
it). Triage: fresh-context Claude subagents, one per finding; one writer applied the
fixes.

| Id | Severity | Verdict | Reason or fix |
| --- | --- | --- | --- |
| C1 | medium | valid | `purpose-and-scope.md` and `repository-map.md` explained the effects boundary; cut to a sentence and a link to Architecture |
| C2 | medium | valid, done outside the pages | The first transcript was not a start-to-finish run in a scratch worktree; the replay above is |
| C3 | low | valid | `terminology.md` called `test` a gate; it is outside `just check` |
| C4 | low | valid | `maintain-dependencies.md` said Python is pinned exactly; it is `3.14.*` |
| B1 | low | valid | The tutorial's prerequisites omitted `just` |
| B2 | low | valid | "All three nextest tests pass" is now "Every nextest test passes" |
| B3 | low | invalid | The fallback command's flags exist and the page already describes the secondary-worktree skip; the evidence gap is recorded above |
| B4 | low | valid | `repository-map.md` said `.pixi/envs/default/bin` is on `PATH` nowhere but recipes; `pixi run` and CI's activated environment put it there too |
| B5 | low | valid | `develop-locally.md`'s list of writing recipes now includes `just lock-upgrade` |
| B6 | low | invalid | Links to stubs that parts 2 and 3 of this ticket replace on the same branch |

No finding was high, so there was no second round. Re-run at `bf51e0d` with part 1
uncommitted after the fixes: `just check` ended `All checks passed and the worktree is
unchanged.` (exit 0); the part-1 acceptance `rg` printed only
`docs/project/purpose-and-scope.md:16`, the sentence naming libpawdoku.

### Adversarial review, checkpoint 2 (`ai_tmp/review/T07-2/`)

Reviewers: Codex `gpt-6-astra`, both lenses, over `git diff d631186` (part 2,
uncommitted). Triage and apply as at checkpoint 1.

| Id | Severity | Verdict | Reason or fix |
| --- | --- | --- | --- |
| C1 / B2 | medium | valid | `api.md` implied T14 might lift `publish = false`; T14 keeps it (`tickets/T14-release.md`), so docs.rs is conditional on a later decision |
| C2 / B3 | low | invalid | `layering.md`'s one-sentence summary links to Architecture; it does not restate the boundary |
| C3 | low | valid | `commands.md` said `just deny` checks every dependency; `deny.toml` sets `exclude-dev = true`, so "every dependency that ships" |
| B1 | medium | valid, part handed back | `quality-gates.md` and `commands.md` said `check-toolchain` proves the pinned toolchain is active; it does not compare them (`RUSTUP_TOOLCHAIN=1.98.1 just check-toolchain` exits 0). The pages now say so; the recipe fix is handed back below |

No finding was high. Re-run at `d631186` with part 2 uncommitted: `just check` ended
`All checks passed and the worktree is unchanged.` (exit 0).

### Adversarial review, checkpoint 3 (`ai_tmp/review/T07-3/`)

Reviewers: Codex `gpt-6-astra`, both lenses, over `git diff 97edd83` (part 3,
uncommitted). Triage and apply as before.

| Id | Severity | Verdict | Reason or fix |
| --- | --- | --- | --- |
| C1 / B1 | medium | valid | 0003 kept L's claim that cancellation cannot be passed as a trait; with threads among the effects it can, so the reopen trigger no longer names it |
| C2 | low | valid | 0005 said `just lock-check` proves every other pin; it proves only the two lockfiles |
| B2 | medium | valid | 0001 said the workspace versions both crates together; each manifest has its own version and T14 tags each crate on its own |

No finding was high. Before the review, the part-3 checker also fixed four small
errors in 0006, 0012 and 0014, and `maintain-dependencies.md` was aligned with 0007 on
where the dependency note goes (beside the range and in the pull request, as L's record
and L's how-to each say one half of).

### Verification, at `d089a0d`

Re-run after the code-review fixes; the only later commit adds these notes. Full output:
`ai_tmp/T07/verification-final2.txt`.

```text
$ just check-docs
Validated 39 pages and 40 canonical topics.                                   exit 0
$ rg -n -i 'pawdoku|sudoku|wasm|no_std' docs/project docs/tutorials docs/how-to
docs/project/purpose-and-scope.md:16:the CI and the Allium tooling, carried from libpawdoku at `ccdd001`, so that the
                                                                              exit 0
$ rg -n -i 'pawdoku|sudoku|wasm|no_std' docs/explanation docs/reference docs/operations
docs/operations/maintenance.md:16:... The routine is the one carried from libpawdoku at
docs/reference/agent-contract.md:75:Seven are the house's own, carried from libpawdoku at `ccdd001` ...
docs/explanation/quality-philosophy.md:15:The argument on this page was carried from libpawdoku at `ccdd001`; ...
docs/explanation/architecture.md:23:libpawdoku at `ccdd001`, a single library crate, and widened to two crates here.
                                                                              exit 0
$ rg -n -i 'sudoku|wasm|no_std' docs/decisions
docs/decisions/0008-std-library.md:20, :21, :34, :40, :99 (why no_std was not chosen)  exit 0
$ ls docs/decisions/000[2-79]-*.md docs/decisions/001[0-3]-*.md | wc -l
      11                                                                      exit 0
$ rg --files-without-match 'Carried from libpawdoku' docs/decisions/000[2-79]-*.md docs/decisions/001[0-3]-*.md
                                                                              exit 1
$ rg -l 'Carried from' docs/decisions/0001-*.md docs/decisions/0008-*.md docs/decisions/0014-*.md
                                                                              exit 1
$ for r in $(just --summary); do ... || echo MISSING $r; done   (every recipe in commands.md)
                                                                              exit 0
$ diff <(the 19 recipes in pyproject.toml) <(rows 1-19 of quality-gates.md)  exit 0
$ just check                                                                  exit 0
All checks passed and the worktree is unchanged.
$ git status --short                                                          exit 0 (empty)
```

`just --list` prints 40 recipes; `default` is covered in prose (a bare `just`), the rest
each have a row. At `012b884`, `RUSTUP_TOOLCHAIN=1.98.1 just check-toolchain` was re-run:
it printed `1.98.1-aarch64-apple-darwin (overridden by environment variable
RUSTUP_TOOLCHAIN)` and exited 0, which is the hand-back below.

### Code review (`/code-review`, `main...HEAD` at `be0c36e`)

Thirteen findings. Applied, one writer, after reading each cited source:

- Gate 8 was described as proving the std-API bans (`quality-gates.md`,
  `architecture.md`, `security-model.md`); today it proves the crate bans only, and the
  std-API bans and probe arrive with T10. The gate 7 paragraph's "have their own probe"
  is corrected the same way.
- "Every gate passes `--locked`" narrowed in `security-model.md`, `repository-map.md`
  and `quality-gates.md` to the recipes decision 0007 names; `deps-unused` does not.
- The `cargo tree` crate check is qualified as covering the host target only
  (`architecture.md`, 0008, `quality-gates.md`). For T10: `--target all` would close it.
- `quality-gates.md`'s gate 8 section no longer restates the ban list; it links
  Architecture.
- The lock-check row now gives the `Justfile`'s exact flags; `configuration.md` names the
  three lints `clippy.toml` relaxes in tests; 0012 names all six jobs that run the setup
  action; `quality-gates.md` says T13's `image` job will join the `check` job's `needs`;
  `testing.md` says `nextest-version` is a minimum, not the pin; `test-and-debug.md`
  names the two workflows that set `RUST_BACKTRACE`.
- The tutorial now restores only the two files it changed, with `git restore`.

Not applied: the branch name (left as the worktree's, as recorded under Baseline), and
the links to T06's pages, which are still T06's stubs (T06's lane).

### PR review (#8)

One finding, from Copilot (low, valid): two passages still said `--locked` is passed by
every recipe, which `just deps-unused`, the snapshot recipes and `just lock` /
`just lock-upgrade` contradict. Both now use decision 0007's wording, "every recipe that
builds, tests, lints or documents the workspace": `maintain-dependencies.md`'s opening
paragraph, which also names the two exceptions, and `troubleshooting.md`'s `--locked`
entry.

At `8334764` (`ai_tmp/T07/pr8-check.txt`):

```text
$ just check
All checks passed and the worktree is unchanged.
```

### Handed back

- **`check-toolchain` does not enforce the pin** (checkpoint 2, B1). The recipe proves
  `cargo` is rustup's proxy and prints the active toolchain, but an override
  (`RUSTUP_TOOLCHAIN`, `+toolchain`, `rustup override`) passes. The `Justfile` is
  frozen in wave 1, so this is a T00 follow-up on `main`: fail when
  `rustup show active-toolchain` does not start with `rust-toolchain.toml`'s `channel`
  or reports an override.
- **`cargo tree` sees the host target only.** `scripts/check-effects.sh`'s crate line
  omits a banned crate behind another platform's target-specific dependency. For T10:
  pass `--target all`.
- **Invariant 8's "every gate passes `--locked`"** is not literally true: `just
  deps-unused` runs `cargo shear` with no `--locked`. Decision 0007 names the recipes
  that do pass it rather than repeating the invariant. For T02 or T11: add the flag if
  cargo-shear takes one, or narrow the invariant in `AGENTS.md`.
- **`AGENTS.md:36` says "No workflow publishes yet"**, but `pages.yml` has deployed from
  `main` since `49890ae`. Not a T07 file; for T11.

### Sentences kept from L that needed a fact checked

Decision records

- 0001: build, test, lint, coverage and doc recipes run with `--workspace`, so the
  sentence names those kinds only (`fmt` uses `--all`; deny, shear and `effects` take no
  flag): `Justfile` lines 100, 106-107, 161, 200, 206.
- 0002: the chain to Pawdoku's 0003 at `78d03cdf`:
  `git -C ~/projects/libpawdoku show ccdd001:docs/decisions/0002-specs-are-the-source-of-truth.md`.
- 0003: L's "`rand` and `getrandom` are banned by cargo-deny" is false here; `deny.toml`
  `[bans]` denies `openssl-sys` alone. `crates/libpachinko/tests/` does not exist (`ls`),
  so the integration test is written as "will".
- 0004: nineteen recipes, not L's seventeen, and no predicates: `pyproject.toml`
  `[tool.biscuit-games-tooling]`. Six console scripts: the installed package's
  `entry_points.txt`. "631 lines" and "592 lines": `wc -l` over the v0.3.0 package. No
  `.py` file ships: `git ls-files '*.py' | wc -l` = 0.
- 0005: Allium 3.6.1 and its four checksums: `install_allium.py` lines 34-60;
  `.tools/bin/allium --version` prints `allium 3.6.1`. That `allium-cli` 3.6.1 is on
  crates.io is L's dated claim, not re-run (no network).
- 0006: no `NOTICE` file (`ls NOTICE*`); T14 keeps `publish = false`:
  `tickets/T14-release.md` lines 21, 42, 88.
- 0007: `--locked` named per recipe, because `cargo shear` (`Justfile:229`) lacks it;
  `just audit` is a local recipe as well as a workflow (`Justfile:223-226`,
  `.github/workflows/audit.yml`); no `.cargo/config.toml` (`ls .cargo` fails).
- 0008: `rust-toolchain.toml` has no `targets` key; `rg 'wasm|no_std' Justfile Cargo.toml
  crates/` is empty; `tests/fixtures/` does not exist, so the probe is written as T10's
  design.
- 0009: the only integration test is `crates/pachinko/tests/binary.rs`
  (`ls crates/*/tests`).
- 0010: vendored byte for byte from L: `git -C <L> archive ccdd001 allium-skill-reference`
  then `diff -r`, identical. L's lock did not hash `SKILL.md`: its `computedHash`
  `52446c61...` differs from `shasum -a 256` of the file (`2445a3b1...`).
- 0011: Python is pinned to `3.14.*`, not exactly, and cargo-insta is in the pixi list:
  `pyproject.toml`. No `pixi run` in the `Justfile`, scripts or workflows:
  `rg 'pixi run'` empty.
- 0012: `excessive_nesting` sits in clippy's `complexity` group and
  `cognitive_complexity` in `restriction`: `clippy-driver -W help`. Four CI jobs run the
  setup action, not L's five: `ci.yml:41,68,91,107`. rustqual's own behaviour at 1.8.3
  (line counting, no release binary, `#[expect]` not charged to the budget) is carried
  from L without a local run.
- 0013: first deployment from `main` at `49890ae` on 2026-10-23, and the environment
  admits `main` alone: `tickets/T01-github-and-ci.md` hand-back. `ci.yml` and
  `audit.yml` hold only `contents: read`: `rg -n -A3 '^permissions' .github/workflows/*.yml`.
- 0014: the crate ban list: `scripts/check-effects.sh` line 8 (`banned=`).
- README: "0004 superseded in part by 0011": L's `0011-tool-manager.md` lines 13, 57 at
  `ccdd001`.

Handbook pages

- `agent-contract.md`: 14 skills and 43 managed files:
  `git ls-files .claude .codex .agents | wc -l`; seven `<important if>` blocks:
  `rg -c '<important if' AGENTS.md`; L's "five byte for byte" dropped as unverifiable.
- `api.md`: the public API is `libpachinko::version` and `pachinko::run`, and only
  `version` has a doctest: `crates/*/src/lib.rs`.
- `commands.md`: every recipe present, `just --summary` against the page; audit runs on
  every pull request as well as weekly: `.github/workflows/audit.yml:10-13`.
- `configuration.md` and `testing.md`: no recipe or workflow selects nextest's `ci`
  profile: `rg -- '--profile|NEXTEST_PROFILE' Justfile .github scripts pyproject.toml`
  empty. `GITHUB_TOKEN` reaches the install-tools step only:
  `.github/actions/setup/action.yml:65-74`.
- `deploy-to-github-pages.md`: the URL comes from T01's read-back of `html_url`; "no
  custom domain" is inferred from it, not checked against the `cname` field.
- `develop-locally.md`: platforms `linux-64` and `osx-arm64`, "win-64 never":
  `pyproject.toml` `platforms` and its comment.
- `documentation-contract.md`: L's rules carry unchanged because the validator is the
  same package at the same tag: `pyproject.toml:35` = L `ccdd001:pyproject.toml:35`.
- `first-change.md`: `--no-fail-fast` exists in the pinned nextest 0.9.146:
  `grep -a -c -- 'no-fail-fast' .pixi/envs/default/bin/cargo-nextest` = 3; panic
  locations and summary lines: `ai_tmp/T07/tutorial-transcript.txt`.
- `layering.md`: Cargo refusing normal-dependency cycles but allowing dev-dependency
  cycles is not verified by command (no cargo runs); the page qualifies it.
- `quality-gates.md`: branch protection (only `check`, not strict, no reviews):
  `tickets/T01-github-and-ci.md` lines 212-213, 248; shellcheck covers both scripts (no
  `files:` filter).
- `quality-philosophy.md` and `test-and-debug.md`: `main.rs` is in the coverage
  denominator (no `--ignore-filename-regex`) and covered by the binary test:
  `tickets/T00*.md:329`, claim 4. `cargo llvm-cov report --html` and
  `--show-missing-lines`: `cargo-llvm-cov llvm-cov report --help` (0.9.1).
- `security-model.md`: only `pages.yml` holds a write scope:
  `rg -n write .github/workflows/`; L's "cargo-hack publishes no checksum" dropped as
  unverifiable.
- `troubleshooting.md`: each quoted message read from its source:
  `run_project_check.py:72,76,113-134`, `validate_docs.py:209,218,319,325`,
  `validate_agents.py:32,148,229,245`, `install_allium.py:207-213`,
  `scripts/check-effects.sh:28`.

### Sentences written to the design, for T11 to reconcile

T10: the effects mechanism and probe

- `docs/decisions/0008-std-library.md:63`: "The std-API bans use the mechanism T10
  chooses"; name the mechanism and its file.
- `docs/decisions/0008-std-library.md:47`: "any crate T10 adds with a reason".
- `docs/decisions/0008-std-library.md:67`: "A ban that is not shown to fire"; confirm the
  fixture layout.
- `docs/decisions/0008-std-library.md:74`: "Today the script runs the crate check alone".
- `docs/decisions/0008-std-library.md:88`: "a ban in the script and a line in the fixture
  that breaks it".
- `docs/decisions/0008-std-library.md:89`: "Until T10 lands, a direct std call in
  libpachinko is caught by review alone".
- `docs/decisions/0008-std-library.md:93`: "The crate line needs no network once"; confirm
  under network denial (CONVENTIONS §12 claim 12).
- `docs/decisions/0001-library-and-service.md:69`: "has to look at the library's own
  dependency tree".
- `docs/decisions/0009-rust-quality-gate.md:23`: "`effects` at gate 8".
- `docs/decisions/0014-service-runtime-deferred.md:53-54`: "the mechanism T10 chooses will
  ban `std::net`" and the crate list "`tokio`, `async-std`, `smol`, `mio` and
  `socket2`".
- `docs/decisions/0012-metrics-gate.md:61`: "runs over `crates/pachinko` too"; mention a
  second rustqual configuration if T10 picks `rustqual-effects.toml`.
- `docs/explanation/architecture.md:85,96,101`: "any crate T10 adds with a reason", "the
  probe arrive with T10", "through the mechanism T10 chooses, and a probe".
- `docs/reference/commands.md:72`: "the std-API bans, the probe over".
- `docs/reference/configuration.md:124`: "The std-API bans arrive by the mechanism T10
  chooses".
- `docs/reference/quality-gates.md:26`: "nothing it calls" (true for calls only after T10).
- `docs/reference/quality-gates.md:99,104`: "The crate bans are checked today." and "by the
  mechanism T10 chooses, arrive with T10".
- `docs/explanation/quality-philosophy.md:40,43,46`: "it already carries a positive
  control", "arrive with the mechanism T10 chooses", "Until the probe lands the script
  says so on every run".
- `docs/project/repository-map.md:43,85`: the `effects-violation/` tree line and table row
  (path does not exist yet).
- `docs/project/repository-map.md:66`: "The configuration of the std-API bans arrives with
  the mechanism T10 chooses"; add the chosen file and drop the sentence.
- `docs/explanation/security-model.md:36,72`: "the fakes that sit beside the effect
  traits" and "`just effects` proves it".
- `docs/how-to/test-and-debug.md:134,138`: "which today fails when libpachinko's normal
  dependency tree names a banned crate" and "arrive with the mechanism T10 chooses".
- `docs/operations/troubleshooting.md:77`: "lists the bans the script checks today".

T10: the metrics configuration

- `docs/decisions/0012-metrics-gate.md:54,97`: "The architecture dimension stays on" and
  the row "Module boundaries | none yet"; every threshold row assumes L's values.
- `docs/decisions/0012-metrics-gate.md:165`: "That change amends this record".
- `docs/reference/commands.md:71`: "Module-boundary rules wait for the first modules".
- `docs/reference/configuration.md:59`: "Architecture is enabled but holds no
  `[[architecture.pattern]]` rule yet".
- `docs/explanation/layering.md:72,74`: "rule per module" and "Today it holds no pattern
  rule, because there is no module to bound."
- `docs/reference/quality-gates.md:73,77`: "The module-boundary rules wait for there to be
  modules to order" and "Nothing proves the complexity, cohesion and coupling sections".
- `docs/project/repository-map.md:24`: "run once per crate".
- `docs/how-to/maintain-dependencies.md:111`: "a rustqual release can move what `just
  metrics` reports".
- `docs/operations/maintenance.md:33`: "because rustqual is a gate".

T02: the lint table, manifests and tests

- `docs/decisions/0009-rust-quality-gate.md:35,40,60`: "`pedantic` and `cargo` groups",
  "`allow_attributes_without_reason` enforce that", "`tests_outside_test_module` lint
  holds the module shape".
- `docs/decisions/0012-metrics-gate.md:37`: the lint table names `cognitive_complexity`.
- `docs/decisions/0007-dependency-policy.md:45,50`: "gate 2" and "gate 13" (gate numbers
  follow `pyproject.toml`'s list).
- `docs/reference/api.md:27`: the lint sentences from `missing_docs` on.
- `docs/reference/api.md:36,71`: "today `libpachinko::version` carries one" (`pachinko::run`
  has no doctest) and "today the public API is two functions".
- `docs/reference/commands.md:70`: "The manifests say `warn`".
- `docs/reference/configuration.md:48`: "There is no `[workspace.dependencies]` table" and
  "fat LTO and one codegen unit".
- `docs/reference/configuration.md:55`: "No recipe or workflow selects `ci` today".
- `docs/reference/configuration.md:99`: "There is no `std_instead_of_core` family".
- `docs/reference/configuration.md:110`: "One crate denied: `openssl-sys`".
- `docs/explanation/layering.md:84,86`: "`unreachable_pub` and `unnameable_types`" and
  "`mod_module_files`".
- `docs/how-to/maintain-dependencies.md:65`: "The first external dependency adds a
  `[workspace.dependencies]` table".
- `docs/reference/quality-gates.md:24`: "Every lint in the workspace table is clean".
- `docs/explanation/quality-philosophy.md:57,77`: `module_name_repetitions` allowed, and
  "Two crates hold a handful of items".
- `docs/project/repository-map.md:23`: "Clippy's thresholds and disallowed types".
- `docs/how-to/test-and-debug.md:49`: the binary test's asserted "`pachinko 0.1.0`" line.
- `docs/reference/testing.md:17,37,61`: "Neither crate has a development dependency yet.",
  "One integration test file exists, and libpachinko has none yet.", "Every public item
  is to carry one."

T06: the specification pages and module

- `docs/decisions/0002-specs-are-the-source-of-truth.md:67`: the Related link to
  `work-with-the-specs.md` (and `specifications.md`).
- `docs/decisions/0005-project-managed-allium-cli.md:88`: "the waiver terms are in"; confirm
  `work-with-the-specs.md` carries them.
- `docs/how-to/maintain-dependencies.md:198` and `docs/reference/quality-gates.md:63`: the
  links to `work-with-the-specs.md` for waivers.
- `docs/explanation/architecture.md:37`: "explains why the specification comes first".
- `docs/explanation/layering.md:31`: "explains how a module states its imports".
- `docs/reference/configuration.md:132`: "Today the one module,
  `docs/specs/pachinko.allium`, declares nothing".
- `docs/operations/maintenance.md:48`: "The modules carry no waiver today".
- `docs/reference/testing.md:82`: "`just plan-spec pachinko` has nothing to ask for".
- `docs/project/terminology.md:37`: "today the only one".
- `docs/tutorials/first-change.md:59`: "today it declares no entity, rule, surface or
  invariant".

T13: the container image

- `docs/decisions/0014-service-runtime-deferred.md:63`: "T13's container image of this
  binary".
- `docs/explanation/architecture.md:140`: "T13 will add a container image".
- `docs/operations/maintenance.md:12`: "deployed nowhere".
- `docs/project/purpose-and-scope.md:44`: "The API reference is the one thing deployed".
- `docs/decisions/0013-api-reference-on-github-pages.md:18,74`: "hold no scope beyond
  `contents: read`" and "The one workflow that holds a write scope" (T13's and T14's
  workflows add write scopes).

T14: the release process

- `docs/decisions/0001-library-and-service.md:65`: "through a registry today".
- `docs/decisions/0006-apache-2-0.md:40`: "The release process that T14 adds in wave 2";
  may link decision 0016.
- `docs/decisions/0007-dependency-policy.md:90`: "the first decision to publish one".
- `docs/decisions/0013-api-reference-on-github-pages.md:66,67,80`: "still to come" and "if
  a crate were ever published".
- `docs/reference/api.md:61`: "release process ticket T14 adds keeps it so".
- `docs/explanation/architecture.md:141`: "T14 will add the release process".
- `docs/how-to/deploy-to-github-pages.md:23`: "the release process arrives with T14".
- `docs/operations/maintenance.md:59`: "so there is no registry token either".
- `docs/project/purpose-and-scope.md:45`: "Wave 2 adds a release process (T14)".
- `docs/explanation/security-model.md:29,77`: "the release process's to decide" and "The
  release workflow T14 designs".

T15 and T16: mutation testing and fuzzing

- `docs/decisions/0009-rust-quality-gate.md:75`: "T15 and T16 add them as recipes outside
  `just check`"; may link decision 0017.
- `docs/reference/quality-gates.md:121`: "Wave 2 (T13 to T17) adds recipes outside `check`
  only".

T17: Renovate

- `docs/decisions/0007-dependency-policy.md:74`: "until ticket T17 brings Renovate".
- `docs/decisions/0012-metrics-gate.md:126`: "Nothing bumps a tool pin automatically
  today."
- `docs/how-to/maintain-dependencies.md:19`: "Renovate takes that over when ticket T17
  lands".
- `docs/operations/maintenance.md:52`: "until ticket T17 brings Renovate".
- `docs/decisions/README.md:53`: "0015 to 0018" reserved for wave 2 (shrinks as T13-T17
  land).

Other

- `docs/decisions/0003-effects-behind-traits.md:66`: "will exercise each trait and its
  fake from outside the crate"; present tense when the first trait lands.
- `docs/project/terminology.md:31,32,34,35`: "no runtime, transport or HTTP stack is
  chosen", "The service is the only one today.", "None exists yet", "kept beside the
  trait in the library".
- `docs/explanation/quality-philosophy.md:85`: "No effect trait exists yet".
- `docs/decisions/0004-hook-runner-and-checkers.md:41`: "the nineteen recipes `just check`
  runs".
- `docs/how-to/develop-locally.md:89`: "as its third gate".
- `docs/decisions/0014-service-runtime-deferred.md:70`: concurrency without a surface is
  left as the maintainer's question, not a trigger.
- `docs/how-to/test-and-debug.md:119` and `docs/reference/testing.md:117`: "The denominator
  is small today" and "moves the percentage by several points".
- `docs/how-to/test-and-debug.md:82` and `docs/reference/testing.md:130`: "no test takes a
  snapshot yet".
- `docs/how-to/test-and-debug.md:42,101`: the Testing links carry no fragment; `testing.md`
  now has `## Snapshots`.
- `docs/operations/troubleshooting.md:58`: the Agent contract link carries no fragment;
  `agent-contract.md` now has `## What a bridge must be`.

### What each carried record lost from L

- 0002: the eight engine modules moved from the game became the one placeholder module;
  the solver argument was rewritten domain-neutrally; the paragraph on the modules being
  read in two repositories went, as there is no second consumer; L's "restated for the
  library" clause became the libpawdoku-to-Pawdoku chain sentence.
- 0003: the `no_std` argument (left to 0008), the `human-solving.allium` quotation, the
  randomness types (`RandomStream`, `SeededStream`, `ReplayStream`), the CLI exemption,
  the false cargo-deny ban on `rand` and `getrandom`, the wasm consequence and L's
  generator golden test all went; replay and step budget were generalised, because no
  trait exists here and the effects gate is 0008's.
- 0004: everything about `uv` (this repository never used it), the `crates/pawdoku-py`
  and maturin argument (no bindings crate), the contributor prerequisites (now 0011's)
  and L's claim of a filed ticket for the phrase list (not verified); "Pawdoku" became
  "the games", and an explicit partial-supersession paragraph was added.
- 0005: L's "restated for the library ... Pawdoku's own record stands" line became the
  chain sentence, and "Pawdoku rejected `cargo install`" was rewritten without the name;
  nothing domain-specific needed removing.
- 0006: L's "first licensed repository in the house" (false: libpawdoku came first) and
  its Python, WebAssembly and game consumers went; `publish = false` is restated as a
  maintainer decision that T14 keeps, not as waiting on a release process.
- 0007: the WebAssembly-poison argument, the founding-ticket lockfile owner, the
  one-lane rule for additions, the dependency-updates spike (now T17), "audit runs in CI
  only", the blanket `--locked` claim (`cargo shear` lacks it), the bindings crate and
  the "one place AGENTS.md departs" claim (false: 0007, 0009, 0010) were all removed or
  narrowed to what this repository shows.
- 0009: `wasm-check` and the wasm targets went (no wasm here); `effects` joined the gate
  list, so "ten of eleven" became "nine of eleven"; L's history of when gates arrived
  went, since all exist from the first commit; Pawdoku crate paths became `crates/*/`;
  the benchmarks spike became T15 and T16.
- 0010: the `skills-lock.json` account went (no lock here, and L's lock hashed the
  upstream folder, not `SKILL.md`); "moved pin" became "next vendoring"; "Pawdoku met
  the same wall" was rewritten without the name; "one directory per skill" now names the
  three that exist.
- 0011: the `wasm32v1-none`/`no_std` argument and its reopen trigger went; L's taplo
  failure is attributed to the carried record; the uv consequences of 0004 are named as
  never holding; "five tools", the exact Python pin, "and its targets" and "every pin is
  a hash" were corrected to this repository's pins; L's 0013 became 0012.
- 0012: the spike id and date, solver wording, L's ten `[[architecture.pattern]]` rules
  and metrics probe, the tree measurements, the "38 seconds" timing, "five gate jobs"
  and the IOSP and budget "today" readings went: they describe L's tree or were not
  re-run here; boundary rules now return with the first two-module specification.
- 0013: L's project-site URL, the game that already deployed to Pages, L's 2026-10-02
  shape decision and "first workflow with a write scope" went or were rewritten for this
  repository's carry decision and first deploy; the root index links both crates, and
  the docs.rs consequence is conditional on publishing.

## Open points

None.
