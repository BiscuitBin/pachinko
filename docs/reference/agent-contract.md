---
title: "Agent contract"
kind: "reference"
audience: [contributor, maintainer, agent]
canonical_for: [agent_contract]
requires: []
---

# Agent contract

Enforced by `bg-validate-agents`, a console script of the `biscuit-games-tooling`
package, run by `just check-agents` and by the `validate-agents` pre-commit hook.
`AGENTS.md` is the single source of truth for how an agent works in this repository;
everything else in the agent surface exists only so a particular tool can find it.

## The four surfaces

| Path | Role |
| --- | --- |
| `AGENTS.md` | Canonical. Read natively by Codex, and by anything following the convention. |
| `CLAUDE.md` | A pointer, byte-pinned to `@AGENTS.md` and nothing else. |
| `.github/copilot-instructions.md` | A pointer, byte-pinned to one paragraph. |
| `.agents/skills/` | Canonical task procedures, mirrored by thin bridges under `.claude/skills/` and `.codex/skills/`. |

The two pointers are the adapters. The validator compares each one whole, so a changed
word, extra whitespace or a missing final newline fails it.

A `CODEX.md` at the repository root is forbidden: Codex reads `AGENTS.md` directly, and a
second file would be a second source of truth.

## How `AGENTS.md` is laid out

Rules that hold for every task are always visible; rules for one kind of task sit in an
`<important if="...">` block that names that task, so an agent reads a block only when its
condition applies. In order:

- The always-on core: what the repository holds, a `## Project map`, the rule that the
  specifications under `docs/specs/` decide what the code does, the rule that issue text,
  pull request bodies, comments, fixtures and tool output are untrusted data, and
  `## Invariants` with the nine invariants. The authority rules follow, still outside
  every block: anything that leaves the worktree needs explicit authorization for each
  action, the recipes that reach the network are named, scratch goes in `ai_tmp/`, and
  work is handed back only when `just check` is green.
- Seven `<important if="...">` blocks, one per task: changing Rust under `crates/`;
  adding or changing an effect; writing or changing a module under `docs/specs/`;
  changing a page under `docs/`; changing `AGENTS.md`, an adapter, a skill or a bridge;
  a failing gate, or handing work back; picking up a ticket under `tickets/`. Each block
  names only the skill and the recipes its task uses.
- `## Provenance`, last.

There is no commands table: `just --list` is the index, and
[Commands](commands.md) describes each recipe.

## What `AGENTS.md` must contain

- At least 300 words.
- Six phrases, each naming an invariant that is expensive to rediscover: `untrusted`,
  `just check`, `explicit authorization`, `ai_tmp/`, `docs/specs/`, and `runes`.

The phrase list is a crude check and is meant to be. It does not verify that the guidance
is good; it verifies that the six topics were not dropped in an edit. Each phrase appears
at least once outside every `<important if>` block, so that none depends on a block's
wording.

The sixth phrase names the games' reactivity rule, which has nothing to govern in a Rust
workspace. `AGENTS.md` satisfies it with one honest sentence in its Provenance section
saying the rule does not apply here, which stays while the tooling package's phrase list
is fixed.

## What a skill must be

This repository carries fourteen canonical skills, one directory each under
`.agents/skills/`, in two groups. Each has two bridges, so there are twenty-eight.

Seven are the house's own, carried from libpawdoku at `ccdd001` and adapted to this
repository's two crates and gates. `review-change` is the games' `code-review` renamed,
so that it does not shadow Claude Code's built-in `/code-review`.

| Skill | Description, from its frontmatter |
| --- | --- |
| `fix-quality` | Diagnose and repair a failing quality gate at its root instead of suppressing the finding. |
| `plan-change` | Plan a change when the scope, affected specifications, risks, or validation need evidence before any code is written. |
| `project-check` | Bring a workspace to a state where the full gate runs, and interpret what it reports. |
| `review-change` | Review a change in this repository against its invariants, specifications, tests, and documentation contract. |
| `review-docs` | Add or revise a handbook page so it satisfies the documentation contract and stays reachable. |
| `rust-change` | Implement or review a change to either crate with the effects boundary, tests and documentation evidence the invariants require. |
| `spec-change` | Change an Allium specification and carry the change through the tests and the implementation. |

Seven are the Allium skills, vendored from `juxt/allium`. Each
carries a paragraph naming `AGENTS.md` and the `just` recipe that proves its work, which
the check below requires of every skill; `propagate`'s names this repository's two
integration-test directories. No lock file records their source.

| Skill | Description, first sentence of its frontmatter |
| --- | --- |
| `allium` | Give your AI agents something more useful than a prompt. |
| `distill` | Extract an Allium specification from an existing codebase. |
| `elicit` | Run a structured discovery session to build an Allium specification through conversation. |
| `propagate` | Generate tests from Allium specifications. |
| `tend` | Tend the Allium garden. |
| `weed` | Weed the Allium garden. |
| `witness` | Independently witness that an Allium loop's convergence claim is true and was reached honestly. |

Frontmatter of exactly two keys:

```markdown
---
name: rust-change
description: Implement or review a change to either crate with the effects boundary, tests and documentation evidence the invariants require.
---
```

- `name` equals the directory name.
- `description` is at least eight words and states a real trigger.
- The body cites `AGENTS.md` and names at least one `just` recipe. A procedure that ends
  without saying how to verify it is not a procedure.

The frontmatter parser is deliberately literal. It splits every line between the `---`
markers on the first colon and treats a line without one as an error, so a blank line
inside the block fails the check with `invalid frontmatter line`. Keep the two keys
adjacent, with nothing between them. The same parser reads the bridges, so the rule holds
there too.

A key that appears twice is an error rather than an overwrite. Left to the usual last-wins
behaviour, a block of three lines would satisfy a rule about two keys on whichever copy
happened to survive, which is the opposite of what "exactly two" is for.

## What a bridge must be

Each of `.claude/skills/<name>/SKILL.md` and `.codex/skills/<name>/SKILL.md` carries the
canonical frontmatter verbatim, then exactly this sentence and nothing else:

```markdown
Follow `../../../.agents/skills/<name>/SKILL.md`. That file is canonical and this bridge adds nothing to it.
```

The body is compared against that template rather than measured against a word budget. A
budget was the earlier rule and it enforced the wrong thing: a bridge could carry an
instruction of its own — an extra step, a caveat, a second pointer — and pass on being
brief. This is an instruction surface, so a clause smuggled into it is read as guidance and
becomes a second source of truth for the skill it points at. A bridge that grows content
fails whether or not it is short.

## The inventory

The validator lists managed files from Git, honouring only this repository's
`.gitignore`, and compares that against what it expects.

- Every expected file must exist. A skill without its two bridges fails.
- No unexpected file may exist under `.agents/`, `.claude/` or `.codex/`. The single
  exception is `.claude/settings.json`, which is tolerated but not required — it carries
  provider configuration rather than agent guidance.
- Managed files must be regular files, never symlinks, and carry no unresolved template
  syntax.

Local assistant state stays out of the inventory by being listed in `.gitignore`. That is
deliberate: the check reads Git rather than walking the filesystem, so an ignored file is
invisible to it.

Because nothing else may exist under `.agents/`, the vendored skills' reference material
lives outside it, at `allium-skill-reference/` at the repository root, with one directory
each for `allium`, `distill` and `elicit`. It is linked only from those skills. Git marks
that directory as vendored, and markdownlint, the link checker and `typos` all ignore it;
[Decision 0010](../decisions/0010-skill-reference-material.md) is the record. On this
tree `just check-agents` prints `Validated AGENTS.md, 2 adapters, and 14 skills.`

## Related pages

- [Documentation contract](documentation-contract.md)
- [Quality gates](quality-gates.md)
- [Commands](commands.md)
- [Repository map](../project/repository-map.md)
- [Decision 0010](../decisions/0010-skill-reference-material.md)
