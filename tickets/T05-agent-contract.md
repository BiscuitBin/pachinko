---
id: T05
title: "Agent contract: AGENTS.md, fourteen skills, the Allium vendoring"
status: open
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

To be written by the executing agent: each adaptation to a vendored skill, with its line;
the upstream source the dropped `skills-lock.json` named;
the facts `docs/reference/agent-contract.md` (T07) needs; the review record for each
checkpoint (`CONVENTIONS.md` §14); anything handed back.

## Open points

None.
