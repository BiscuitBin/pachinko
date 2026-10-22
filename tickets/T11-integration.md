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
recorded outcome; every check a T00 to T10 hand-back says it did not run, skipped or only
dry-ran has a recorded disposition, and each one left open has an owning ticket; and the
handbook says what landed. Claims 7, 8 and 9 belong to T14,
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
| `tickets/T18-verification-gaps.md` | new, only if an item stays open | one item per open row of step 5's second table |
| `tickets/README.md` | correction | T18's index row and graph entry, only if T18 is written |
| any lane file a hand-back names | correction | the reconciled sentence |
| `tickets/T11-integration.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T01, T02, T05, T06, T07 and T10 have merged.
2. **Reconcile.** Read every lane's hand-back notes. For each item handed back or written
   to the design, find it on `main` and make it true; list each with its resolution.
   Also list every check a hand-back says it did not run, skipped, or ran only as a dry
   run (`just -n`, a documentation read, a scratch copy); step 5 disposes of them.

   **Adversarial review** (`CONVENTIONS.md` §14): the reconciliation list, with every
   lane's hand-back notes beside it in the brief instead of a diff. Ask: which item
   handed back, or sentence written to the design, is missing from the list; which
   resolution is claimed but not on `main`.
3. **Fresh clone from the remote.** `git clone https://github.com/BiscuitBin/pachinko
   ai_tmp/fresh`, then `just initialize` (network, authorised) and `just check` there.
   Quote the whole `check` output. Then run `just check` again in the same clone under
   `CONVENTIONS.md` §11's network denial, and quote it with both negative-control exit
   codes. If the gate needs the network, that is a finding to hand back; do not edit a
   frozen file to make it pass.
4. **CI on `main`, and branch protection.** Dispatch `ci.yml` and `audit.yml` on `main`
   and read both runs. Read branch protection back with
   `gh api repos/BiscuitBin/pachinko/branches/main/protection/required_status_checks --jq '[.checks[] | {context, app_id}]'`
   and confirm it prints one entry, `check`, with the GitHub Actions `app_id` T01
   recorded. `.contexts` is deprecated and is not the evidence. Diff the whole protection
   response, normalised with `jq -S .` as T01 saved it, against the one T01 quoted after
   its apply; any field that changed since is
   a finding.
5. **The claims table.** One row per `CONVENTIONS.md` §12 claim 1 to 6 and 10 to 12 with
   its outcome, the ticket that checked it and where the evidence is quoted; claims 7 to
   9 stay open, marked as T17's. Then a second table, "Checks the hand-backs did not
   run", from step 2's list: Source ticket, Item, Disposition. A disposition is Closed
   (cite where the evidence is quoted) or Open. A green `just check` or CI run is cited
   only for an item it actually executes. Likely rows, as examples and not a checklist:
   the primary checkout's hook shim refusing a bad commit; `snapshots-review` and
   `snapshots-accept`, which have nothing to act on before the first snapshot;
   initialization with no cache reused; `audit.yml` on `main`. Open rows that no existing
   ticket owns go into one new ticket, `tickets/T18-verification-gaps.md` (the next free
   id; check the README index on `main`), in this directory's ticket format, with one
   item per row stating exactly what evidence closes it, and recording it as open rather
   than inferring success from an unrelated green gate. It depends on T11 and is off
   the critical path. Carry the claims table into `CONVENTIONS.md` §12, and add T18 to
   the README index and graph, through one `main` pull request, after the review below:
   **authorisation required: push and open a pull request**.

   **Adversarial review** (`CONVENTIONS.md` §14): the claims table, before its `main`
   pull request is asked for. Ask: which outcome is not backed by evidence quoted in a
   ticket's hand-back notes; which outcome claims more than its quoted evidence shows;
   which check a hand-back did not run is missing from the second table; which
   disposition cites a gate that does not run it.
6. **Cleanliness.** `rg -n -i 'stub|lands with T|arrive with T|TODO'` over `docs/`,
   `AGENTS.md`, `.agents/` and the crates prints only sentences that name a wave-2 ticket
   or the specifications; `rg -n -i 'pawdoku|sudoku|wasm'` outside `tickets/` and
   `allium-skill-reference/` prints only carried-from sentences and `AGENTS.md`'s
   Provenance sentence naming libpawdoku as the source.
7. Delete `ai_tmp/fresh`, set `status: done`, commit.

## Acceptance criteria

- The fresh-clone `just check` is green and quoted, and green again under
  `CONVENTIONS.md` §11's network denial with both negative-control probes failing.
- CI on `main` is green, and branch protection requires `check`, read back from
  `.checks` with the GitHub Actions `app_id`.
- Every `CONVENTIONS.md` §12 claim 1 to 6 and 10 to 12 has an outcome; claims 7 to 9
  are marked open for T17.
- Every check a T00 to T10 hand-back did not run has a row in step 5's second table; no
  green gate is cited for an item it does not run; open rows without an owner are items
  in `tickets/T18-verification-gaps.md`, which is in the README index.
- Step 6's searches print only what the hand-back notes justify.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just check
# macOS; on Linux, unshare -rn in place of sandbox-exec (CONVENTIONS.md §11)
deny='(version 1)(allow default)(deny network*)'
sandbox-exec -p "$deny" curl -fsS --max-time 10 https://static.rust-lang.org; echo "exit $?"
sandbox-exec -p "$deny" curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1; echo "exit $?"
sandbox-exec -p "$deny" just check; echo "exit $?"
gh run list --repo BiscuitBin/pachinko --branch main --limit 3
gh api repos/BiscuitBin/pachinko/branches/main/protection/required_status_checks --jq '[.checks[] | {context, app_id}]'
rg -n -i 'stub|lands with T|arrive with T|TODO' docs AGENTS.md .agents crates
```

## Hand-back notes

To be written by the executing agent: the reconciliation list, the fresh-clone output,
the offline gate output and both probe exits, the run URLs, the protection read-back and
its diff against T01's, the claims table, the table of checks the hand-backs did not
run and T18's id if one was opened, and the review record for each checkpoint
(`CONVENTIONS.md` §14).

## Open points

None.
