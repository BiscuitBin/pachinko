---
id: T05
title: "Agent contract: AGENTS.md, fourteen skills, the Allium vendoring"
status: done
depends_on: [T00]
parallel_with: [T01, T02, T06, T07, T10]
branch: ticket/t05-agent-contract
estimated_size: M
---

# T05: Agent contract: AGENTS.md, fourteen skills, the Allium vendoring

## Context

T00 left an `AGENTS.md` stub carrying the six phrases B requires, fourteen skill stubs
and twenty-eight frozen bridges. This lane writes the contract for a two-crate std
project whose domain the specifications will define: L `AGENTS.md` and L's fourteen
skills, adapted as `CONVENTIONS.md` §7 states, plus the vendored Allium material.

L `tickets/T05-agent-contract.md` is the procedure. B
`src/biscuit_games_tooling/validate_agents.py` is what the result must satisfy: the six
required phrases (lines 47-54), the skill frontmatter, the bridge body and the adapters.

## Goal

`AGENTS.md` states this repository's invariants (`CONVENTIONS.md` §7) and nothing L-only;
`AGENTS.md` has the shape `CONVENTIONS.md` §7 states; the seven house skills are adapted
to two crates and the effects boundary; the seven Allium skills and
`allium-skill-reference/` are vendored from L; there is no `skills-lock.json`; and
`just check-agents` prints `Validated AGENTS.md, 2 adapters, and 14 skills.`

## Non-goals

- No change to `CLAUDE.md`, `.github/copilot-instructions.md`, `.claude/settings.json` or
  any bridge (frozen), and none to any skill's frontmatter, which the bridges repeat and
  T00 wrote final (`CONVENTIONS.md` §7).
- No new skill and no removed skill: the fourteen are fixed by T00's stubs.
- No handbook page. `docs/reference/agent-contract.md` is T07's; this lane hands it the
  facts it needs in the hand-back notes.
- No domain guidance. Where L's skills name sudoku modules, the adaptation names the
  placeholder module or the general case.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `AGENTS.md` | final | L's content, restructured per `CONVENTIONS.md` §7 |
| `.agents/skills/rust-change/SKILL.md` | final | body: two crates, the effects boundary |
| `.agents/skills/spec-change/SKILL.md` | final | one placeholder module |
| `.agents/skills/plan-change/SKILL.md` | final | L's |
| `.agents/skills/review-change/SKILL.md` | final | invariants of `CONVENTIONS.md` §7 in the checklist |
| `.agents/skills/review-docs/SKILL.md` | final | L's |
| `.agents/skills/fix-quality/SKILL.md` | final | one dispatch line for each of the nineteen gates of `CONVENTIONS.md` §4 |
| `.agents/skills/project-check/SKILL.md` | final | gate numbers of `CONVENTIONS.md` §4; green as `CONVENTIONS.md` §11 defines it |
| `.agents/skills/allium/SKILL.md` | final | L's |
| `.agents/skills/distill/SKILL.md` | final | L's |
| `.agents/skills/elicit/SKILL.md` | final | L's |
| `.agents/skills/propagate/SKILL.md` | final | L's, with the test locations of both crates |
| `.agents/skills/tend/SKILL.md` | final | L's |
| `.agents/skills/weed/SKILL.md` | final | L's |
| `.agents/skills/witness/SKILL.md` | final | L's |
| `allium-skill-reference/` | new | L `allium-skill-reference/`, verbatim |
| `tickets/T05-agent-contract.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. **`AGENTS.md`.** Start from L `AGENTS.md` and restructure it as `CONVENTIONS.md` §7
   states: the always-on core, the authority rules, the seven `<important if>` blocks,
   then Provenance. Every L sentence that states an invariant, an authority boundary or
   a deviation lands in the core or in a block; the rest is cut. The identity is a
   library and a service with an undefined domain; the project map says where each kind
   of test lives. The hand-back rule reads: rerun the one recipe `just check` named and
   read only its first failure; hand back only when `just check` ends with
   `All checks passed and the worktree is unchanged.` and `git status --short` is empty.
   In Provenance, keep the runes sentence (L `AGENTS.md` line 188 onwards) and name L at
   `ccdd001` as the source. Every recipe named must exist in the `Justfile`.
3. **The house skills.** Adapt each from L's, per `CONVENTIONS.md` §7. `rust-change` is
   the one with new substance: a new effect is a trait in libpachinko with a fake beside
   it, and its real implementation in pachinko; `just effects` proves the boundary.
   `fix-quality` keeps L's dispatch lines and adds one for each gate L's lacks, including
   gate 7 `metrics` (lower the complexity, never a `rustqual.toml` threshold) and gate 8
   `effects` (an effect reached directly goes behind a libpachinko trait with a fake
   beside it; a probe failure on the fixture is a defect in `scripts/check-effects.sh`,
   never in the fixture). `project-check`'s last step confirms green as
   `CONVENTIONS.md` §11 defines it, in place of L's bare `just check-clean`, which with
   no baseline fails on any uncommitted change. Neither skill says `just initialize`
   formats (`CONVENTIONS.md` §5).
4. **The Allium skills.** Copy the seven from L. `allium/SKILL.md` names recipes that
   exist here unchanged; `propagate/SKILL.md` line 12 names `crates/pawdoku/tests/`, which
   becomes the test directories of both crates. Change nothing else, so the vendored text
   stays comparable to upstream.
5. **The vendoring.** Copy L `allium-skill-reference/` verbatim. Do not create
   `skills-lock.json` (`CONVENTIONS.md` §7). Record in the hand-back notes, for T07's
   decision 0010 and `docs/reference/agent-contract.md`, the upstream source the lock
   named (`juxt/allium`).
   Confirm `.gitattributes`, `.markdownlint-cli2.jsonc`, `lychee.toml` and `_typos.toml`
   already exclude `allium-skill-reference/` (T00 copied L's); if not, hand it back as a T00 follow-up.
6. `just check-agents`, `just check-docs`, `just check`, the final adversarial review,
   `just check` again if fixes landed, then commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff. Ask: whether
   `AGENTS.md` carries the nine invariants of `CONVENTIONS.md` §7 verbatim and the six
   phrases of §1 fact 2; whether any authority rule, or any of the six phrases, sits
   only inside an `<important if>` block; which block names a recipe or skill its task
   does not use; which recipe a file names that the `Justfile` does not define; whether
   a vendored skill changed beyond the lines step 4 states.

## Acceptance criteria

- `just check-agents` prints `Validated AGENTS.md, 2 adapters, and 14 skills.`
- `rg -n -i '(^|[^b])pawdoku|sudoku|wasm|no_std' AGENTS.md .agents/` prints nothing. The
  pattern skips `libpawdoku`, which step 2's Provenance names as the source.
- `rg -c '^<important if="' AGENTS.md` prints `7`, and each of B's six phrases appears
  outside every block (the phrase loop in Verification prints nothing).
- Every gate of `CONVENTIONS.md` §4 appears as a code span in
  `.agents/skills/fix-quality/SKILL.md` (the loop in Verification prints nothing).
- `diff -r` against L's `allium-skill-reference/` is empty.
- `skills-lock.json` does not exist.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just check-agents
rg -n -i '(^|[^b])pawdoku|sudoku|wasm|no_std' AGENTS.md .agents/
mkdir -p ai_tmp/l
git -C /Users/scutting/projects/libpawdoku archive ccdd001 allium-skill-reference | tar -x -C ai_tmp/l
diff -r ai_tmp/l/allium-skill-reference allium-skill-reference; echo "exit $?"
test ! -e skills-lock.json; echo "exit $?"
rg -c '^<important if="' AGENTS.md
awk '/^<important if=/{b=1} !b{print} /^<\/important>/{b=0}' AGENTS.md > ai_tmp/agents-core.md
for p in untrusted 'just check' 'explicit authorization' ai_tmp/ docs/specs/ runes; do
  rg -qiF "$p" ai_tmp/agents-core.md || echo "only in a block: $p"
done
for g in check-toolchain lock-check lint fmt-check toml-check clippy metrics effects features test-doc coverage doc deny deps-unused check-docs check-agents check-specs analyse-specs snapshots-check; do
  rg -qF "\`$g\`" .agents/skills/fix-quality/SKILL.md || echo "missing $g"
done
just check
git status --short
```

## Hand-back notes

### Baseline

At `8b01c0c` (untouched worktree), `just check` ended with
`All checks passed and the worktree is unchanged.` (exit 0).

### Adaptations to the vendored skills

- `propagate/SKILL.md` line 12: `integration tests under \`crates/pawdoku/tests/\`` became
  `integration tests under \`crates/libpachinko/tests/\` and \`crates/pachinko/tests/\``.
- Nothing else changed: `allium`, `distill`, `elicit`, `tend`, `weed` and `witness` are
  byte-identical to L at `ccdd001`, and each one's frontmatter equals T00's stub. Every
  recipe they name (`check-specs`, `analyse-specs`, `test`, `check`, `plan-spec`)
  already exists in the `Justfile`.
- `allium-skill-reference/` is L's at `ccdd001`, 23 files, `diff -r` empty.

### The dropped `skills-lock.json`

L's lock named `source: juxt/allium`, `sourceType: github`, and a `skillPath` of
`skills/<name>/SKILL.md` for each of the seven skills, with no tag and no commit. It is
not carried (`CONVENTIONS.md` §7). For decision 0010 and T07: the upstream is the GitHub
repository `juxt/allium`, and the version the vendored text came from is whatever L held
at `ccdd001`.

### Facts for T07 (`docs/reference/agent-contract.md`, decision 0010)

- `AGENTS.md` order: identity paragraphs (single source of truth; library and service
  with an undefined domain; specifications decide *what*; the untrusted-data rule),
  `## Project map` (including where tests live), `## Invariants` (the nine of §7
  verbatim; one sentence saying invariant 4 is enforced by the workspace lint table every
  crate inherits; the authority rules; the change-workflow rules; the hand-back rule),
  seven `<important if>` blocks, `## Provenance`. About 1300 words, 866 outside the
  blocks.
- The seven blocks, in order: Rust under `crates/` (`rust-change`, `review-change`);
  an effect (`rust-change`, `just effects`); a module under `docs/specs/`
  (`spec-change`, which hands the language to `allium`); a page under `docs/`
  (`review-docs`); the agent surface; a failing gate or hand-back (`fix-quality`,
  `project-check`); picking up a ticket (`plan-change`).
- The network recipes `AGENTS.md` names, all ten in the `Justfile` that reach the
  network: `initialize`, `install-toolchain`, `install-tools`, `install-allium`, `sync`,
  `lock`, `lock-upgrade`, `install-hooks`, `audit`, `check-links-online`. The stub named
  eight; `sync` and `lock` are added.
- The authority rule covers destructive operations inside the worktree as well as
  every action that leaves it.
- No workflow publishes today (`ci.yml` is read-only); `AGENTS.md` says that once one
  deploys from `main` (decision 0013), a push there is a deployment.
- Skills: fourteen canonical under `.agents/skills/`, twenty-eight bridges, frontmatter
  untouched since T00. Reference material sits at `allium-skill-reference/`, linked only
  from the Allium skills. The dotfile exclusions T00 copied are in place:
  `.gitattributes:5`, `.markdownlint-cli2.jsonc:17`, `lychee.toml:6`, `_typos.toml:14`.

### Adversarial review, checkpoint 1 (`ai_tmp/review/T05-1/`)

Reviewers: Codex CLI 0.163.0, `codex exec -m gpt-6-astra -s read-only`, one run per
lens over `git diff main...HEAD` at `3156a65`. Triage: a fresh-context Claude Opus
subagent, which applied the fixes in `c3b8ed0`.

| Id | Severity | Verdict | Reason or fix |
| --- | --- | --- | --- |
| C01 / B1 | medium / high | valid | L's rule that destructive operations need authorization was dropped; restored in the authority paragraph |
| C02 | medium | invalid | L's snapshot sentence is a stack convention, not an invariant, authority rule or deviation; passed to T07 below |
| C03 / B2 | low / medium | valid | `AGENTS.md` said `spec-change` hands off to `allium`, but the skill did not; its step 3 now does |
| B3 | medium | valid | `rust-change` blocked ticket-scoped scaffolding that no clause can govern yet; it may now cite the ticket, and domain behaviour still needs a clause |
| B4 | medium | valid, handed back | The Verification block's archive pipeline can diff against a stale extraction; the fix is in this ticket's Verification block (below) |

Round 2 ran because B1 was high: the same reviewers over `git diff HEAD~1 HEAD` at
`c3b8ed0`. Both lenses reported no findings.

### Code review (`/code-review`, medium, `main...HEAD` at `c3b8ed0`)

Ten findings. Applied in `1657dff`:

- `rust-change` and `review-change` overclaimed `just effects`; the gate bans crates
  today, so both now say to read the diff for a direct `std` call as well.
- `fix-quality`'s `lint` line now routes a hook named after a gate to that gate's line.
- `fix-quality` marks `just lock` and `just install-tools` as needing authorization.
- `AGENTS.md`: the snapshot sentence no longer calls the snapshot recipes "the one
  exception to `--locked`"; the specs block says the recipes run the pinned checker and
  a bare `allium` on `PATH` is not trusted; L's rules on adding a missing recipe to the
  `Justfile`, reading only the skill the task needs, and linking the reference material
  only from the Allium skills are restored.

Not applied:

- This ticket's `status:` and hand-back notes were still unwritten: done in this commit.
- `fix-quality` names the fixture `tests/fixtures/effects-violation`, which T10 creates:
  the ticket's step 3 requires that wording, and the `Justfile` comment names it too.
- Handed back, below: `.allium-loop/`, `deps-unused` and `--locked`, the stub waiver
  page, and the vendored skills' bare `allium` advice.

### Verification, at `1657dff` (no uncommitted changes)

The block above, run as written except that `just check`'s output goes to a file so its
own exit status is quoted (the plain `| tail` form hides it). Each line's exit status is
in brackets.

```text
$ just check-agents
Validated AGENTS.md, 2 adapters, and 14 skills.                     [exit 0]
$ rg -n -i '(^|[^b])pawdoku|sudoku|wasm|no_std' AGENTS.md .agents/
(nothing)                                                           [exit 1]
$ mkdir -p ai_tmp/l                                                 [exit 0]
$ git -C /Users/scutting/projects/libpawdoku archive ccdd001 allium-skill-reference | tar -x -C ai_tmp/l
                                                                    [exit 0]
$ diff -r ai_tmp/l/allium-skill-reference allium-skill-reference; echo "exit $?"
exit 0
$ test ! -e skills-lock.json; echo "exit $?"
exit 0
$ rg -c '^<important if="' AGENTS.md
7
$ awk ... AGENTS.md > ai_tmp/agents-core.md                         [exit 0]
$ for p in untrusted ... runes; do ...; done
(nothing)                                                           [exit 0]
$ for g in check-toolchain ... snapshots-check; do ...; done
(nothing)                                                           [exit 0]
$ just check
All checks passed and the worktree is unchanged.                    [exit 0]
$ git status --short
(nothing)                                                           [exit 0]
```

Positive controls: the Pawdoku `rg` over L's `AGENTS.md` and `.agents/` matches 22
lines. ai_tmp/l was emptied before this run. The fail-closed form of the vendoring check
(B4) at `c3b8ed0`: `git archive -o` exit 0, `tar -x -f` exit 0, 23 files, `diff -r` exit 0.

### Handed back

- **This ticket's Verification block (B4).** It should run
  `rm -rf ai_tmp/l && mkdir -p ai_tmp/l`, then `git archive -o` and `tar -x -f` as two
  commands, each with its exit status echoed, before the `diff -r`.
- **T00 follow-up: `.allium-loop/` in `.gitignore`.** The vendored loop
  (`allium-skill-reference/allium/driving-the-loop.md` line 92) writes its ledger under
  `.allium-loop/` and tells the agent to add it to `.gitignore`. `.gitignore` is frozen,
  so a `/allium` loop today leaves the worktree dirty.
- **Invariant 8 against `deps-unused`.** Invariant 8 (§7, verbatim) says every gate
  passes `--locked`, but `deps-unused` runs `cargo shear`, which takes no such flag. For
  T07's quality-gates page, or a §7 wording change.
- **T07: `docs/how-to/work-with-the-specs.md`.** `spec-change` step 7 (L's text) points
  there for the `allium-ignore` waiver terms; the page is a stub until T07 writes it.
- **T07: the snapshot rule.** L's sentence "Snapshots show a reviewer a value and detect
  change; they never prove a clause." belongs on `docs/reference/testing.md`.
- **Vendored skills and a bare `allium`.** `weed`, `tend`, `elicit`, `distill` and
  `witness` say to run `allium` "if the CLI is available", and `weed` suggests installing
  it. They stay verbatim, as step 4 requires; `AGENTS.md`'s specs block points agents to
  the pinned checker instead.
- **`AGENTS.md` length.** About 1300 words against §7's "about 1000": the authority list,
  the network recipe list and the review fixes account for the excess.
- **Not run:** `git fetch` and the rebase onto `origin/main` that §11 asks for before a
  push, because no push was authorised. `main` is `8b01c0c`, which this branch is based on.

## Open points

None.
