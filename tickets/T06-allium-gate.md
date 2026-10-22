---
id: T06
title: "Allium gate: the placeholder module, the spec recipes, the two spec pages"
status: open
depends_on: [T00]
parallel_with: [T01, T02, T03, T04, T05, T07, T08, T09, T10]
branch: ticket/t06-allium-gate
estimated_size: S
---

# T06: Allium gate: the placeholder module, the spec recipes, the two spec pages

## Context

The specifications come after this foundation, but the gate that checks them comes now,
so that the first real module lands into a working loop. B `src/biscuit_games_tooling/run_allium.py`
line 131 refuses an empty `docs/specs/`, so T00 shipped `docs/specs/pachinko.allium`, a
root module that declares no behaviour (`CONVENTIONS.md` §1 fact 1, §9). This lane makes
it final, proves every Allium recipe on it, and writes the two pages that explain the
specification workflow.

L `tickets/T06-spec-migration.md` is the nearest procedure, though it migrated seven real
modules; its hand-back notes record allium 3.6.1's behaviour. L
`docs/how-to/work-with-the-specs.md` and L `docs/explanation/specifications.md` are the
pages this lane adapts. L `tickets/T24-plan-spec-recipe.md` is the `plan-spec` recipe's
origin.

## Goal

`just install-allium`, `just check-specs`, `just analyse-specs` and `just plan-spec
pachinko` are each green on the placeholder, which is final and says plainly that it
holds no behaviour yet; the zero-diagnostics, zero-findings result is quoted; and the two
spec pages describe the workflow the first real module will follow.

## Non-goals

- No behaviour in the module: no entity, rule, surface, invariant or config that a test
  would have to honour.
- No second module.
- No recipe change; a flag found wrong is a T00 follow-up.
- No handbook page beyond the two this lane owns.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | final | `CONVENTIONS.md` §9 |
| `docs/how-to/work-with-the-specs.md` | final | L's, the module table with one row, the waiver terms verbatim |
| `docs/explanation/specifications.md` | final | L's argument; one module that states nothing yet |
| `tickets/T06-allium-gate.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. **The module.** Read T00's hand-back notes for the outcome of `CONVENTIONS.md` §12
   claim 1. Finalise the header comments: Scope (the domain is undefined; this is its
   root), Includes (nothing yet), Excludes (every behaviour until a specification change
   defines one), Dependencies (none), Vocabulary (none yet; `docs/project/terminology.md`
   is the glossary). Keep `-- allium: 3`.
3. **The recipes.** `just install-allium` (network, authorised), confirm the binary is
   3.6.1 with B's checksum, then `just check-specs`, `just analyse-specs` and
   `just plan-spec pachinko`. Quote each.
4. **The gate fires.** In a scratch commit, add a malformed line to the module and confirm
   `just check-specs` fails; add a rule with an unreachable condition and confirm
   `just analyse-specs` reports a finding. Revert both. This shows the empty-result green
   is not a silent pass.
5. **`work-with-the-specs.md`.** L's page, with the module table reduced to the one
   placeholder, the waiver terms verbatim (L's "Diagnostics and waivers" section), and a
   short section on adding the first module: where it goes, how the root imports it, and
   which skill drives it (`spec-change`, then `propagate`).
6. **`specifications.md`.** L's argument that the specifications decide behaviour; the
   state today (one module, no behaviour) and what changes when the first lands. Cite
   decision 0002.
7. `just check-docs`, `just check`, then commit.

## Acceptance criteria

- `just check-specs` and `just analyse-specs` each print one block with empty
  `diagnostics` and `findings`.
- `just plan-spec pachinko` exits 0; its output is quoted.
- Step 4's two failures are quoted.
- `just check` is green.

## Verification

```sh
.tools/bin/allium --version
just check-specs
just analyse-specs
just plan-spec pachinko
just check-docs
just check
```

## Hand-back notes

To be written by the executing agent: the binary version and checksum, each recipe's
output, the two proofs that the gate fires, and the outcome of `CONVENTIONS.md` §12
claim 1.

## Open points

None.
