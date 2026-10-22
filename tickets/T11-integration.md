---
id: T11
title: "Integration: the first green `just check` from a fresh clone and in CI, branch protection verified"
status: open
depends_on: [T01, T02, T03, T04, T05, T06, T07, T08, T09, T10]
parallel_with: []
branch: ticket/t11-integration
estimated_size: M
---

# T11: Integration: the first green `just check` from a fresh clone and in CI, branch protection verified

## Context

Ten lanes have merged, each green on its own branch, each writing some sentences to the
design while another lane settled the fact (T08 and T09 waited on T10's mechanism; T05
handed facts to T08). Several will have handed back follow-ups to frozen files. This
ticket is where the pieces are proved to fit: the follow-ups reconciled, every sentence
written to the design checked against what landed, and the whole gate run from a fresh
clone of the remote and in CI, with branch protection shown to block a red pull request.

L `tickets/T11-integration.md` is the procedure; its eight steps carry over with this
repository's names.

## Goal

`main` on `BiscuitBin/pachinko`, cloned fresh from GitHub, passes `just initialize` then
`just check`; CI on `main` and on a pull request is green; branch protection requires
`check` and refuses to merge a pull request whose `check` is red; every
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

1. Create the worktree after T01 to T10 have merged.
2. **Reconcile.** Read every lane's hand-back notes. For each item handed back or written
   to the design, find it on `main` and make it true; list each with its resolution.
3. **Fresh clone from the remote.** `git clone https://github.com/BiscuitBin/pachinko
   ai_tmp/fresh`, then `just initialize` (network, authorised) and `just check` there.
   Quote the whole `check` output.
4. **CI on `main`.** Dispatch `ci.yml` and `audit.yml` on `main` and read both runs.
5. **Branch protection, proved.** **Authorisation required: push a branch and open a
   pull request.** A throwaway branch that breaks one gate (an unformatted line); confirm
   `check` goes red and `gh pr merge` refuses without an administrator bypass; then close
   the pull request and delete the branch (each separately authorised).
6. **The claims table.** One row per `CONVENTIONS.md` §12 claim 1 to 6 and 10 to 12 with
   its outcome, the ticket that checked it and where the evidence is quoted; claims 7 to
   9 stay open, marked as T17's. Carry it into `CONVENTIONS.md` §12 through a `main` pull
   request.
7. **Cleanliness.** `rg -n -i 'stub|lands with T|arrive with T|TODO'` over `docs/`,
   `AGENTS.md`, `.agents/` and the crates prints only sentences that name a wave-2 ticket
   or the specifications; `rg -n -i 'pawdoku|sudoku|wasm'` outside `tickets/` and
   `allium-skill-reference/` prints only carried-from sentences.
8. Delete `ai_tmp/fresh`, set `status: done`, commit.

## Acceptance criteria

- The fresh-clone `just check` is green and quoted.
- CI on `main` is green; the red pull request could not merge.
- Every `CONVENTIONS.md` §12 claim 1 to 6 and 10 to 12 has an outcome; claims 7 to 9
  are marked open for T17.
- Step 7's searches print only what the hand-back notes justify.

## Verification

```sh
just check
gh run list --repo BiscuitBin/pachinko --branch main --limit 3
gh api repos/BiscuitBin/pachinko/branches/main/protection --jq '.required_status_checks.contexts'
rg -n -i 'stub|lands with T|arrive with T|TODO' docs AGENTS.md .agents crates
```

## Hand-back notes

To be written by the executing agent: the reconciliation list, the fresh-clone output,
the run URLs, the red pull request's number and refusal, and the claims table.

## Open points

None.
