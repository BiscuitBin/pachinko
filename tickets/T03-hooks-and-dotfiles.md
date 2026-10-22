---
id: T03
title: "Hooks and dotfiles: final configs, the first-run script, a fresh clone that works"
status: open
depends_on: [T00]
parallel_with: [T01, T02, T04, T05, T06, T07, T08, T09, T10]
branch: ticket/t03-hooks-and-dotfiles
estimated_size: S
---

# T03: Hooks and dotfiles: final configs, the first-run script, a fresh clone that works

## Context

T00 copied L's dotfiles and both prek configurations so that `just lint` passes. The
prek configurations are frozen; the files they read are this lane's. L
`tickets/T03-hooks-and-dotfiles.md` is the procedure, and its hand-back notes record what
L proved about prek, lychee and the offline first run; this lane re-proves the parts
that could differ in a new repository (paths, names, the worktree's location) and does
not re-derive the rest.

## Goal

`just initialize` from a fresh clone installs everything and `just lint` then passes
with the network blocked; every dotfile is final, with nothing in it that names sudoku,
pawdoku, Node, Svelte or wasm; and the `pyproject.toml` comments describe this
repository.

## Non-goals

- No change to `.pre-commit-config.yaml` or `.pre-commit-fix.yaml` (frozen), or to any
  pin or the recipe list in `pyproject.toml`.
- No new hook. A hook this repository needs is a T00 follow-up.
- No `taplo.toml` (T02).

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `.editorconfig` | final | L's, with only this repository's file types |
| `.gitattributes` | final | L's lines for `Cargo.lock`, `pixi.lock`, `allium-skill-reference/**` |
| `.gitignore` | final | L's, with the `mutants.out*/` comment reworded for T15 |
| `.markdownlint-cli2.jsonc` | final | L's |
| `lychee.toml` | final | L's |
| `_typos.toml` | final | `CONVENTIONS.md` §5 |
| `scripts/initialize.sh` | final | L's, with this repository's names |
| `pyproject.toml` | comments only | the pixi manifest's comments |
| `tickets/T03-hooks-and-dotfiles.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. Diff each file against L at `ccdd001` and against what this repository holds. Remove
   every line that serves a file type or tool this repository does not have; keep every
   comment that explains a setting this repository still uses, reworded where it names
   L. `rg -n -i 'pawdoku|sudoku|svelte|node|wasm|\.py\b'` over the lane's files lists
   the candidates.
3. `_typos.toml`: keep `mis`; drop `als` unless `just check-docs` then fails on a file that
   needs it.
4. `.gitignore`: L's `mutants.out*/` comment says "if S03 adopts it"; reword it to name
   T15. Add nothing a wave-2 ticket will add itself (the fuzz artefacts are T16's).
5. **The fresh clone, offline.** Clone this branch into `ai_tmp/fresh`, run
   `just initialize`, then `HTTP_PROXY=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9
   just lint`. It must not fetch. Delete the clone afterwards.
6. **The secondary worktree.** From a second worktree whose primary has run
   `just initialize`, `just lint` works without a second `install-hooks`, as L's
   `scripts/initialize.sh` comment claims.
7. `just check`, the final adversarial review, `just check` again if fixes landed, then
   commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff. Ask: which removed
   line something in this repository, a later lane or a wave-2 ticket still needs;
   whether step 5's lint was offline, or passed only because a cache was warm.

## Acceptance criteria

- `just lint` passes offline in a fresh clone after `just initialize`.
- The step-2 `rg` prints nothing over this lane's files, or only lines the hand-back
  notes justify.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just initialize
HTTP_PROXY=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9 just lint
rg -n -i 'pawdoku|sudoku|svelte|wasm' .editorconfig .gitattributes .gitignore .markdownlint-cli2.jsonc lychee.toml _typos.toml scripts/initialize.sh pyproject.toml
just check
```

## Hand-back notes

To be written by the executing agent: each removed line and why, the offline lint output,
the review record for each checkpoint (`CONVENTIONS.md` §14), and anything a frozen file
needs (handed back to T00).

## Open points

None.
