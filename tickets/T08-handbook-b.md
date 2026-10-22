---
id: T08
title: "Handbook B: explanation, reference and operations pages"
status: open
depends_on: [T00]
parallel_with: [T01, T02, T03, T04, T05, T06, T07, T09, T10]
branch: ticket/t08-handbook-b
estimated_size: L
---

# T08: Handbook B: explanation, reference and operations pages

## Context

T00 registered every page and left a stub for each. This lane writes the four explanation
pages, the seven reference pages and the two operations pages `CONVENTIONS.md` §6 assigns
to T08, adapted from L's pages of the same path. The architecture page carries the
argument this repository is built on: a library that performs no effect, and a service
that supplies them.

Several facts these pages state are settled by other wave-1 lanes running at the same
time (T02's lint changes, T05's skills, T10's effects mechanism). Write from
`CONVENTIONS.md` and L; where a lane's outcome could change a sentence, write the
sentence to the design and list it in the hand-back notes for T11 to reconcile.

L `tickets/T08-handbook-b.md` is the procedure.

## Goal

Thirteen final pages that describe the repository after wave 1, with the effects
boundary explained once (in `architecture.md`) and referred to everywhere else.
`just check-docs` is green.

## Non-goals

- No page outside this lane's thirteen; no manifest change (frozen).
- No domain words; no runtime, transport or HTTP stack described as chosen
  (decision 0014).
- No wave-2 tool described as existing.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/explanation/architecture.md` | final | the split, the boundary, what is deferred |
| `docs/explanation/layering.md` | final | crate direction; module direction later |
| `docs/explanation/quality-philosophy.md` | final | L's argument, this gate's examples |
| `docs/explanation/security-model.md` | final | supply chain, unsafe, cargo-deny, the boundary |
| `docs/reference/commands.md` | final | every recipe of the `Justfile` |
| `docs/reference/configuration.md` | final | every configuration file |
| `docs/reference/testing.md` | final | nextest, doctests, the binary test, coverage, fakes, snapshots |
| `docs/reference/quality-gates.md` | final | the nineteen gates |
| `docs/reference/documentation-contract.md` | final | L's, verbatim |
| `docs/reference/agent-contract.md` | final | fourteen skills |
| `docs/reference/api.md` | final | rustdoc for both crates |
| `docs/operations/maintenance.md` | final | L's routine |
| `docs/operations/troubleshooting.md` | final | L's, minus wasm |
| `tickets/T08-handbook-b.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. **`architecture.md`.** Two crates and why (decision 0001). libpachinko holds the
   domain once specified and performs no effect: the clock, randomness, the filesystem,
   the network, the environment and threads reach it through traits it defines, with
   fakes beside them for tests (decision 0003). pachinko implements those traits and is
   the only place effects happen. Gate 8 enforces it (decision 0008); name the bans as
   `CONVENTIONS.md` §9 lists them, and the mechanism as "T10's", to be named by T11. What
   is deferred: the runtime, transport and HTTP stack (decision 0014), the container,
   release and bindings.
3. **`layering.md`.** Cargo enforces `pachinko -> libpachinko` and never the reverse.
   Inside libpachinko, module direction will mirror the specification import graph, and
   rustqual's pattern rules (decision 0012) will hold it once there are modules.
4. **`quality-philosophy.md`.** L's argument verbatim where it holds; L's sudoku examples
   replaced with this gate's: the effects probe, the binary test, the coverage floor on a
   small denominator.
5. **The reference pages.** `commands.md` lists every recipe in the `Justfile` with its
   comment's meaning (`just --list` is the check); `quality-gates.md` the nineteen gates
   of `CONVENTIONS.md` §4 in order; `configuration.md` every configuration file in
   `CONVENTIONS.md` §3; `testing.md` per L, plus the binary test and the insta rule from
   L `tickets/T29-snapshot-harness.md` (a snapshot shows a value to a reviewer; it never
   proves a clause) with the note that `insta` arrives with the first snapshot.
6. **The operations pages.** L's, with this repository's names; troubleshooting loses
   the wasm entries.
7. `just check-docs`, `just check`, then commit.

## Acceptance criteria

- `just check-docs` is green.
- Every recipe `just --list` prints appears in `docs/reference/commands.md`, and every
  gate in `CONVENTIONS.md` §4 in `docs/reference/quality-gates.md`, in order.
- `rg -n -i 'pawdoku|sudoku|wasm|no_std' docs/explanation docs/reference docs/operations`
  prints nothing outside the two pages T06 owns, or only a sentence naming L as the
  source.
- `just check` is green.

## Verification

```sh
just check-docs
just --list
rg -n -i 'pawdoku|sudoku|wasm|no_std' docs/explanation docs/reference docs/operations
just check
```

## Hand-back notes

To be written by the executing agent: every sentence written to the design that another
lane's outcome could change (for T11), and anything handed back.

## Open points

None.
