---
id: T05
title: "Agent contract: AGENTS.md, fourteen skills, the Allium vendoring"
status: open
depends_on: [T00]
parallel_with: [T01, T02, T03, T04, T06, T07, T08, T09, T10]
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
the seven house skills are adapted to two crates and the effects boundary; the seven
Allium skills, `skills-lock.json` and `allium-skill-reference/` are vendored from L; and
`just check-agents` prints `Validated AGENTS.md, 2 adapters, and 14 skills.`

## Non-goals

- No change to `CLAUDE.md`, `.github/copilot-instructions.md`, `.claude/settings.json` or
  any bridge (frozen).
- No new skill and no removed skill: the fourteen are fixed by T00's stubs.
- No handbook page. `docs/reference/agent-contract.md` is T08's; this lane hands it the
  facts it needs in the hand-back notes.
- No domain guidance. Where L's skills name sudoku modules, the adaptation names the
  placeholder module or the general case.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `AGENTS.md` | final | L's, adapted per `CONVENTIONS.md` §7 |
| `.agents/skills/rust-change/SKILL.md` | final | two crates, the effects boundary |
| `.agents/skills/spec-change/SKILL.md` | final | one placeholder module |
| `.agents/skills/plan-change/SKILL.md` | final | L's |
| `.agents/skills/review-change/SKILL.md` | final | invariants of `CONVENTIONS.md` §7 in the checklist |
| `.agents/skills/review-docs/SKILL.md` | final | L's |
| `.agents/skills/fix-quality/SKILL.md` | final | gate names of `CONVENTIONS.md` §4 |
| `.agents/skills/project-check/SKILL.md` | final | gate numbers of `CONVENTIONS.md` §4 |
| `.agents/skills/allium/SKILL.md` | final | L's |
| `.agents/skills/distill/SKILL.md` | final | L's |
| `.agents/skills/elicit/SKILL.md` | final | L's |
| `.agents/skills/propagate/SKILL.md` | final | L's, with the test locations of both crates |
| `.agents/skills/tend/SKILL.md` | final | L's |
| `.agents/skills/weed/SKILL.md` | final | L's |
| `.agents/skills/witness/SKILL.md` | final | L's |
| `skills-lock.json` | new | L `skills-lock.json`, verbatim |
| `allium-skill-reference/` | new | L `allium-skill-reference/`, verbatim |
| `tickets/T05-agent-contract.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. **`AGENTS.md`.** Start from L `AGENTS.md`. Keep its section order. Replace the
   Invariants with `CONVENTIONS.md` §7's nine; rewrite "What this project is" for a
   library and a service with an undefined domain; in "Stack and conventions", two
   crates, std, the effects boundary and where each kind of test lives; in Provenance,
   keep the runes sentence (L `AGENTS.md` line 188 onwards) and name L at `ccdd001` as the
   source. Every recipe named must exist in the `Justfile`.
3. **The house skills.** Adapt each from L's, per `CONVENTIONS.md` §7. `rust-change` is
   the one with new substance: a new effect is a trait in libpachinko with a fake beside
   it, and its real implementation in pachinko; `just effects` proves the boundary.
4. **The Allium skills.** Copy the seven from L. `allium/SKILL.md` names recipes that
   exist here unchanged; `propagate/SKILL.md` line 12 names `crates/pawdoku/tests/`, which
   becomes the test directories of both crates. Change nothing else, so the vendored text
   stays comparable to upstream.
5. **The vendoring.** Copy L `skills-lock.json` and L `allium-skill-reference/` verbatim.
   Confirm `.gitattributes`, `.markdownlint-cli2.jsonc`, `lychee.toml` and `_typos.toml`
   already exclude `allium-skill-reference/` (T00 copied L's); if not, hand it to T03.
6. `just check-agents`, `just check-docs`, `just check`, then commit.

## Acceptance criteria

- `just check-agents` prints `Validated AGENTS.md, 2 adapters, and 14 skills.`
- `rg -n -i 'pawdoku|sudoku|wasm|no_std' AGENTS.md .agents/` prints nothing.
- `diff -r` against L's `allium-skill-reference/` and `skills-lock.json` is empty.
- `just check` is green.

## Verification

```sh
just check-agents
rg -n -i 'pawdoku|sudoku|wasm|no_std' AGENTS.md .agents/
mkdir -p ai_tmp/l
git -C /Users/scutting/projects/libpawdoku archive ccdd001 allium-skill-reference skills-lock.json | tar -x -C ai_tmp/l
diff -r ai_tmp/l/allium-skill-reference allium-skill-reference && diff ai_tmp/l/skills-lock.json skills-lock.json
just check
```

## Hand-back notes

To be written by the executing agent: each adaptation to a vendored skill, with its
line; the facts `docs/reference/agent-contract.md` (T08) needs; anything handed back.

## Open points

None.
