---
id: T11
title: "Integration: the first green `just check` from a fresh clone and in CI, branch protection read back"
status: open
depends_on: [T01, T02, T05, T06, T07, T10]
parallel_with: []
branch: ticket/t11-integration
estimated_size: M
---

# T11: Integration: the first green `just check` from a fresh clone and in CI, branch protection read back

## Context

Six lanes have merged, each green on its own branch, each writing some sentences to the
design while another lane settled the fact (T07 waited on T10's mechanism; T05 handed
facts to T07). Several will have handed back follow-ups to frozen files. This
ticket is where the pieces are proved to fit: the follow-ups reconciled, every sentence
written to the design checked against what landed, and the whole gate run from a fresh
clone of the remote and in CI, with branch protection read back.

L `tickets/T11-integration.md` is the procedure; its steps carry over with this
repository's names.

## Goal

`main` on `BiscuitBin/pachinko`, cloned fresh from GitHub, passes `just initialize` then
`just check`; CI on `main` is green; branch protection requires `check`; every
`CONVENTIONS.md` §12 claim owned by T00 to T10 (claims 1 to 6 and 10 to 12) has a
recorded outcome; and the handbook says what landed. Claims 7, 8 and 9 belong to T14,
T16 and T13, which start after this ticket; T17 records them.

## Non-goals

- No new feature, recipe, page or dependency. A gap found here is a fix to what a lane
  delivered, or a new ticket.
- No wave-2 work.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/explanation/architecture.md` | correction | the effects mechanism T10 chose |
| `docs/decisions/0008-std-library.md` | correction | the same |
| `docs/reference/quality-gates.md` | correction | gate 8 as built |
| `docs/reference/configuration.md` | correction | the files T10 added |
| `tickets/CONVENTIONS.md` | correction | §9 and §12 outcomes, through a `main` pull request |
| any lane file a hand-back names | correction | the reconciled sentence |
| `tickets/T11-integration.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T01, T02, T05, T06, T07 and T10 have merged.
2. **Reconcile.** Read every lane's hand-back notes. For each item handed back or written
   to the design, find it on `main` and make it true; list each with its resolution.

   **Adversarial review** (`CONVENTIONS.md` §14): the reconciliation list, with every
   lane's hand-back notes beside it in the brief instead of a diff. Ask: which item
   handed back, or sentence written to the design, is missing from the list; which
   resolution is claimed but not on `main`.
3. **Fresh clone from the remote.** `git clone https://github.com/BiscuitBin/pachinko
   ai_tmp/fresh`, then `just initialize` (network, authorised) and `just check` there.
   Quote the whole `check` output.
4. **CI on `main`, and branch protection.** Dispatch `ci.yml` and `audit.yml` on `main`
   and read both runs. Read branch protection back with
   `gh api repos/BiscuitBin/pachinko/branches/main/protection --jq '.required_status_checks.contexts'`
   and confirm it prints `["check"]`, the context the `ci.yml` run reports.
5. **The claims table.** One row per `CONVENTIONS.md` §12 claim 1 to 6 and 10 to 12 with
   its outcome, the ticket that checked it and where the evidence is quoted; claims 7 to
   9 stay open, marked as T17's. Carry it into `CONVENTIONS.md` §12 through a `main` pull
   request.

   **Adversarial review** (`CONVENTIONS.md` §14): the claims table, before its `main`
   pull request is asked for. Ask: which outcome is not backed by evidence quoted in a
   ticket's hand-back notes; which outcome claims more than its quoted evidence shows.
6. **Cleanliness.** `rg -n -i 'stub|lands with T|arrive with T|TODO'` over `docs/`,
   `AGENTS.md`, `.agents/` and the crates prints only sentences that name a wave-2 ticket
   or the specifications; `rg -n -i 'pawdoku|sudoku|wasm'` outside `tickets/` and
   `allium-skill-reference/` prints only carried-from sentences and `AGENTS.md`'s
   Provenance sentence naming libpawdoku as the source.
7. Delete `ai_tmp/fresh`, set `status: done`, commit.

## Acceptance criteria

- The fresh-clone `just check` is green and quoted.
- CI on `main` is green, and branch protection requires `check`.
- Every `CONVENTIONS.md` §12 claim 1 to 6 and 10 to 12 has an outcome; claims 7 to 9
  are marked open for T17.
- Step 6's searches print only what the hand-back notes justify.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just check
gh run list --repo BiscuitBin/pachinko --branch main --limit 3
gh api repos/BiscuitBin/pachinko/branches/main/protection --jq '.required_status_checks.contexts'
rg -n -i 'stub|lands with T|arrive with T|TODO' docs AGENTS.md .agents crates
```

## Hand-back notes

To be written by the executing agent: the reconciliation list, the fresh-clone output,
the run URLs, the protection read-back, the claims table, and the review record for each
checkpoint (`CONVENTIONS.md` §14).

## Open points

None.
