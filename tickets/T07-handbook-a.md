---
id: T07
title: "Handbook A: project, tutorial and how-to pages"
status: open
depends_on: [T00]
parallel_with: [T01, T02, T03, T04, T05, T06, T08, T09, T10]
branch: ticket/t07-handbook-a
estimated_size: M
---

# T07: Handbook A: project, tutorial and how-to pages

## Context

T00 registered every page in `docs/manifest.yml` and left a stub for each. This lane
writes the eight project, tutorial and how-to pages `CONVENTIONS.md` §6 assigns to T07,
each adapted from L's page of the same path. Handbook B (T08) writes the explanation,
reference and operations pages in parallel; T06 owns the two specification pages.

L `docs/reference/documentation-contract.md` is the rulebook (frontmatter equals the
manifest entry, one owning page per topic, 40 words minimum, links that resolve with
exact case). L `tickets/T07-handbook-a.md` is the procedure.

## Goal

Eight final pages that describe this repository as it is after wave 1: a two-crate std
workspace whose domain is undefined, with the effects boundary, the gate and the tools L
has. `just check-docs` is green.

## Non-goals

- No page outside this lane's eight, no manifest change, no new page (frozen).
- No domain words. `docs/project/terminology.md` seeds the glossary with the foundation's
  own vocabulary and nothing the specifications will define.
- No description of a wave-2 tool as if it existed. A page may say what wave 2 adds,
  in a sentence, naming the ticket.

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
| `tickets/T07-handbook-a.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. For each page, read L's page of the same path at `ccdd001` and `CONVENTIONS.md` §6's
   adaptation note. Keep L's structure and every sentence that is still true; rewrite
   what names sudoku, pawdoku, wasm, `no_std`, L's modules or L's URLs.
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
5. **`deploy-to-github-pages.md`.** The `pages.yml` T04 writes and the `site` recipe;
   the URL is the one `gh api repos/BiscuitBin/pachinko/pages --jq .html_url` prints after
   T04, written here as `https://biscuitbin.github.io/pachinko/` with a note to confirm it.
6. `just check-docs`, `just check`, then commit.

## Acceptance criteria

- `just check-docs` is green.
- `rg -n -i 'pawdoku|sudoku|wasm|no_std|steven-cutting/libpawdoku' docs/project docs/tutorials docs/how-to/develop-locally.md docs/how-to/maintain-dependencies.md docs/how-to/test-and-debug.md docs/how-to/deploy-to-github-pages.md`
  prints nothing, or only a sentence naming L as the source.
- The tutorial was followed end to end, and the hand-back notes quote it.
- `just check` is green.

## Verification

```sh
just check-docs
rg -n -i 'pawdoku|sudoku|wasm|no_std' docs/project docs/tutorials docs/how-to
just check
```

## Hand-back notes

To be written by the executing agent: the tutorial transcript, each sentence kept from L
that needed a fact checked and how it was checked, and anything handed back.

## Open points

None.
