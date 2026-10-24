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

Times are UTC, 2026-10-23. `main` was `46c2e33` (PR #9's merge) throughout. The work is
on the local branch `T11-integration`, which Supacode created; the ticket's `branch:`
is `ticket/t11-integration`, the name it is pushed under. Every lane's frontmatter reads
`status: done`; T07's branch was `ticket/07-handbook`, as its notes say.

### Baseline

At `46c2e33`, after `just initialize` in this worktree (exit 0, `Ready. Next: just
check.`): `just check` exit 0, last line `All checks passed and the worktree is
unchanged.`

### Reconciliation list (step 2)

Each lane's hand-back notes and open points were read in full, with every sentence a
lane wrote that rests on another lane's fact. Each item was found on `main` at
`46c2e33` by one extraction pass and one independent verification pass per lane. The
lists below give each item's resolution. "Tn:L" cites a line of `tickets/Tn-*.md` at
`46c2e33`.

**Made true by this ticket.**

| Item | Source | Resolution |
| --- | --- | --- |
| Gate 8 as T10 built it: crate line on every target and feature; clippy's disallowed paths in `crates/libpachinko/clippy.toml`; the four checks; the probe; what review still covers | T10:885-922 ("For T11"); T07:549-590; T00:396-407 | `docs/explanation/architecture.md`, `docs/decisions/0008-std-library.md`, `docs/reference/quality-gates.md` (row 8, the metrics paragraph, the Gate 8 section), `docs/reference/configuration.md` (two new file rows and sections) |
| The other "arrive with T10" sentences | T07:549-590 (and the T05 and T10 notes) | `docs/explanation/quality-philosophy.md`, `docs/explanation/security-model.md`, `docs/how-to/test-and-debug.md`, `docs/operations/troubleshooting.md` (with an entry for the std-API and probe failures), `docs/project/repository-map.md`, `docs/reference/commands.md`, `docs/decisions/0014-service-runtime-deferred.md` |
| "Any crate T10 adds with a reason": T10 added none | T07:553 | Dropped from `architecture.md` and 0008 |
| The skills still said the std-API bans arrive later, and that a probe failure is always a script defect | T05:218-219, :231-232; T10:906-907 | `.agents/skills/rust-change/SKILL.md`, `.agents/skills/review-change/SKILL.md`, `.agents/skills/fix-quality/SKILL.md` |
| `AGENTS.md`: "No workflow publishes yet" | T05:190-191; T07:458-459 | `pages.yml` deploys on every push to `main`, so a push there is a deployment |
| The setup action names L's ticket T03, and says no later step reaches the network | T00:435-436, :450-451 (to T01, not acted on) | `.github/actions/setup/action.yml` comments |
| `.allium-loop/` is not ignored, so an Allium loop dirties the worktree | T05:276-279 (a T00 follow-up) | `.gitignore` line, and its mention in `configuration.md` |
| `module_name_repetitions` is still described as allowed | T02:280-281; T07:631-632 | Row deleted from `configuration.md`; `quality-philosophy.md`'s example is now `multiple_crate_versions` |
| The `cargo` group "requires" the package metadata | T02:282-284 | `configuration.md` says why the fields are kept |
| `forbid` and `deny` "fail everywhere"; taplo sorts "every" dependency table; `_typos.toml` allowlist | T02 code review (T02:269, :276); T10:818 | `configuration.md` |
| The inner `#![allow(..., reason)]` gap | T02:302-307 | `0009` and `configuration.md`'s lint row say review holds that form |
| `layering.md` sends the reader to the wrong page for imports | T07:648 (T06's fact) | Links `work-with-the-specs.md#add-the-first-behavioural-module` |
| Two links without a fragment | T07:719-722 | `test-and-debug.md` and `troubleshooting.md` |
| `.config/nextest.toml` names "T01's workflows" as the `ci` profile's selector | T02:264 | "no recipe or workflow selects it yet" |
| §9's crate line, mechanism, fallback clause and T00's script | T00:403-405; T10:78-81, :273-278; T06:149-150 | `tickets/CONVENTIONS.md` §9: the widened line, the outcome paragraph, the fallback settled |
| §12 outcomes for claims 1 to 6 and 10 to 12 | T11 step 5 | `tickets/CONVENTIONS.md` §12, an Outcome column (below) |

**True on `main` already** (checked at the cited line, no change): T00's F1 script half,
the typos `--force-exclude` fix, the 0012 renumbering, the record audiences, the exact
network list and invariant 4's sentence (T05), the `effects` comment against T10's
script, the `deny.toml` and `rustqual.toml` removals, the `important` element; T01's
open points (administrators not bound, vulnerability reporting enabled, Pages on the
free plan, so no `if: false`), the `app_id`, the `ci.yml` comment rewrite and the action's
`name:`; T02's `wildcards`, `exclude-dev`, Windows-target, licence-allowance,
`definition_in_module_root` and profile comments, and the `insta` rule in `testing.md`;
T05's waiver terms and snapshot rule on T07's pages, decision 0010's facts, the
dotfile exclusions, every recipe the skills name, and the `AGENTS.md` sentences that rest
on T00, T02 and T06; T06's every page and link T07 wrote against it; T07's metrics
sentences, T01 facts and wave-2 sentences, which are true until their wave-2 ticket
lands.

**Left for the maintainer** (a decision, not a correction T11 may make; nothing applied):

- CI runs no `check-clean`, so a gate job that rewrites a tracked file passes CI (T01:443-447).
- `concurrency` cancels an in-progress run on `main` (T00:452-453, kept from L).
- T's `bootstrap_repo.sh` defects found at T01's checkpoint 1 (T01:418-420), an upstream report.
- PR #5 (T01's Pages evidence) has no recorded push authorisation; the maintainer merged it.
- `tickets/README.md`'s Status column reads `open` for every done lane; the README says
  frontmatter is authoritative, and T11 may touch only T18's row.
- Invariant 8 says every gate passes `--locked`; `deps-unused` and the snapshot recipes
  cannot (T05:280-282; T07:454-457). Decision 0007 and `quality-gates.md` already narrow
  it; the invariant is §7's verbatim text.
- `AGENTS.md` is about 1300 words against §7's "about 1000" (T05:291-292).
- Whether a gate should reject the inner `#![allow]` form (T02:302-307).
- The release profile's fat LTO was to be measured by T13, which has no measuring step (T02:296-300).
- The nextest `ci` profile no recipe selects, and `nextest-version` kept equal to the pin by nothing (T02:264, :196).
- The crate bans are a denylist; T10 proposes an allowlist of libpachinko's tree (T10:915-922).
- `std::env::consts` stays reachable, against §9's "everything under `std::env`" (T10:938-943).
- Std code behind another target's `cfg`, and windows-only std paths, are not linted (T10:944-947).
- libpachinko's own tests are held to the bans too (T10:970-974); the handbook now says so.
- T10 edited the frozen `_typos.toml` (`ede`) on its branch (T10:818), and lanes merged
  `main` rather than rebasing (T10:160-162).
- `check-toolchain` does not enforce the pin (T07:445-450); the frozen `Justfile`'s
  `effects` comment still says "the bans ... are the script's", and it and
  `scripts/initialize.sh` say "components and targets" with no target listed (T00:437-440).
- `CONVENTIONS.md` text this ticket left as the historical design, outside §9 and §12:
  §1 fact 4 and §3 (the mechanism as T10's open choice), §2 and §4 ("verbatim", with the
  renumbered decision references, T00:338-340), §3's `module_name_repetitions` and
  `cargo` group wording (T02:280-284), §7's word count, §10's stub job, §11's rebase rule,
  §13's toolchain bump (which should re-run the std inventory, T10:948-950).
- The analyser checks producers only, so a green `analyse-specs` does not prove a guard
  is satisfiable (T06:267-272); `work-with-the-specs.md` does not say so.
- Several T07 sentences go stale when T13, T14 or T17 lands, on pages those tickets do
  not list (T07:659-700): 0006, 0007, 0012, 0013, 0014, `api.md`,
  `deploy-to-github-pages.md`, `purpose-and-scope.md`, `security-model.md`.

**Owned by a wave-2 ticket:** the root documents and crate READMEs (T12), the `SECURITY.md`
pointer to vulnerability reporting (T12), the newer action releases (T17), and the T07
sentences T13, T14 and T17 list.

### Fresh clone (step 3)

At `46c2e33`, 23:37:35Z: `git clone https://github.com/BiscuitBin/pachinko ai_tmp/fresh`
exit 0, HEAD `46c2e33`. `just initialize` exit 0; it installed the hook (this clone is its
own primary checkout) and downloaded allium 3.6.1
(`downloading https://github.com/juxt/allium-tools/releases/download/v3.6.1/allium-aarch64-apple-darwin.tar.gz`,
`installed allium 3.6.1`). `just check`, one line per recipe (the full 315-line log is
`ai_tmp/t11/fresh.log`, uncommitted):

```text
==> just check-toolchain   1.99.1-aarch64-apple-darwin (overridden by '…/ai_tmp/fresh/rust-toolchain.toml')
==> just lock-check        i Dry-run: lock file would not change
==> just lint              every hook Passed
==> just fmt-check         cargo fmt --all --check
==> just toml-check        taplo lint: found files total=26 excluded=1
==> just clippy            Finished `dev` profile
==> just metrics           ::notice::Quality score: 100.0% (2 functions analyzed), then (4 functions analyzed)
==> just effects           effects: libpachinko names none of 8 banned crates and uses none of 121 banned std APIs;
                           effects: the fixture trips every ban exactly once, and pachinko trips none.
==> just features          Finished `dev` profile
==> just test-doc          test result: ok.
==> just coverage          3 tests run: 3 passed; TOTAL … 17 0 100.00% lines
==> just doc               Generated …/target/doc/libpachinko/index.html and 1 other file
==> just deny              bans ok, licenses ok, sources ok
==> just deps-unused       ✓ no issues found
==> just check-docs        Validated 39 pages and 40 canonical topics.
==> just check-agents      Validated AGENTS.md, 2 adapters, and 14 skills.
==> just check-specs       allium check: 1 specifications, no diagnostics and no findings.
==> just analyse-specs     allium analyse: 1 specifications, no diagnostics and no findings.
==> just snapshots-check   info: no unreferenced snapshots found; info: no snapshots to review
==> just check-clean       The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
check exit 0
$ git status --short
(nothing)                                                             exit 0
```

The same clone under the denial, at `46c2e33`, 23:38Z
(`deny='(version 1)(allow default)(deny network*)'`; log `ai_tmp/t11/fresh-offline.log`):

```text
$ sandbox-exec -p "$deny" curl -fsS --max-time 10 https://static.rust-lang.org
curl: (6) Could not resolve host: static.rust-lang.org                exit 6
$ sandbox-exec -p "$deny" curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1
curl: (7) Failed to connect to 1.1.1.1 port 443 after 0 ms: Couldn't connect to server
                                                                      exit 7
$ curl --noproxy '*' -fsS -o /dev/null --max-time 10 https://1.1.1.1  (no sandbox, the control)
                                                                      exit 0
$ sandbox-exec -p "$deny" just check
(the same twenty recipes, each with the line above; effects and deny offline included)
All checks passed and the worktree is unchanged.                      exit 0
$ git status --short
(nothing)                                                             exit 0
```

The gate needs no network. The clone reused the host's rustup toolchain and its Cargo,
pixi and prek caches; only `.pixi/`, `.tools/` and `target/` were new.

Further checks in the same clone, a primary checkout with the hook installed, on a
scratch branch deleted afterwards (log `ai_tmp/t11/fresh-hooks.log`, at `46c2e33`):

```text
$ printf 'this is not allium\n' >> docs/specs/pachinko.allium; git add …; git commit -m 'probe: malformed spec'
Specification diagnostics...........Failed  (hook id: check-specs, exit code: 1)
  docs/specs/pachinko.allium:22:1: error: parse error: expected declaration (…), found 'this'
Specification analysis..............Failed  (hook id: analyse-specs, exit code: 1)
commit exit 1                         (HEAD still 46c2e33)
$ printf '\nThe recieve path.\n' >> docs/README.md; git add …; git commit -m 'probe: typo'
typos...............................Failed  (exit code: 2)
  error: `recieve` should be `receive`  ╭▸ docs/README.md:68:5
commit exit 1
$ cmp docs/README.md <copy taken before the commit>
cmp exit 0                            (typos reported and did not rewrite)
$ git status --short                  (after reset, branch deleted)
(nothing)                             exit 0
$ shasum -a 256 .tools/bin/allium
f41d5cb75cee110d6cb1b92f6d146ae84eacc5929aa28f65380e8f072152b2fd   (T06's quoted binary)
```

### CI on `main` and branch protection (step 4)

Dispatch authorised by the maintainer at 23:52Z. Both ran on `46c2e33`:

```text
$ gh workflow run ci.yml --repo BiscuitBin/pachinko --ref main      exit 0
https://github.com/BiscuitBin/pachinko/actions/runs/40042029974
  workflow_dispatch, success: rust, coverage, deny, documents, check
$ gh workflow run audit.yml --repo BiscuitBin/pachinko --ref main   exit 0
https://github.com/BiscuitBin/pachinko/actions/runs/40042032474
  workflow_dispatch, success: audit
  cargo deny --locked check advisories
  advisories ok
```

The `push` run on the same commit, `40040212316` (CI), and `40040212387` (Pages), also
succeeded. Read back at 23:37:21Z:

```text
$ gh api repos/BiscuitBin/pachinko/branches/main/protection/required_status_checks --jq '[.checks[] | {context, app_id}]'
[{"app_id":15368,"context":"check"}]                                  exit 0
$ gh api repos/BiscuitBin/pachinko/branches/main/protection > protection-now.json   exit 0
$ diff <(jq -S . T01's ai_tmp/protection-after.json) <(jq -S . protection-now.json)
(nothing)                                                             exit 0
```

One entry, `check`, `app_id` 15368 (GitHub Actions, as T01 recorded). T01 saved the
whole response after its apply at 01:11:57Z, in its own worktree's `ai_tmp/`; normalised
with `jq -S .`, it equals today's response field for field, so nothing changed.
Pages also reads `{"build_type":"workflow","cname":null,"html_url":"https://biscuitbin.github.io/pachinko/","https_enforced":true}`
(23:59:21Z), so there is no custom domain.

### The claims table (step 5)

Carried into `tickets/CONVENTIONS.md` §12 as an Outcome column:

| # | Outcome | Checked by | Evidence |
| --- | --- | --- | --- |
| 1 | Holds; no fallback declaration | T00, T06 | T00's claims table; T06:149-180; both recipes in T11's offline clone |
| 2 | Clippy half holds; rustqual half not settled and moot (rejected) | T10 | T10:178-271 (Step 2) |
| 3 | Holds on 1.99.1; L's own tree not linted | T00, T02 | T00:316-321; T02:170 |
| 4 | Holds, no fallback | T00, T02 | T00:329; T02:171 |
| 5 | Holds; the uncached fetch is the stub run `39874411413` | T01 | T01:274-291 |
| 6 | Holds with repository admin rights | T01 | T01:238-272; protection unchanged (above) |
| 7, 8, 9 | Open; T17 records them | T14, T16, T13 | - |
| 10 | Holds, and the gate fires on a stray `.snap` | T00, T02 | T00:330; T02:172 |
| 11 | Holds | T00, T10 | T00:331; T10:705 |
| 12 | Holds on macOS: `just effects` cold, and the whole `just check` | T00, T10, T11 | T10:856-873 (`2ebacac`); step 3 above (`46c2e33`) |

### Checks the hand-backs did not run (step 5)

| Source ticket | Item | Disposition |
| --- | --- | --- |
| T01, T10 | Baseline or final `just check` not run, or run only on an uncommitted tree; no rebase onto `main` before push (T01:202-203; T02:286-290; T05:293-294; T06:323-329; T10:847-883) | Closed: the fresh clone of `46c2e33`, which contains every lane, is green (step 3) |
| T00, T02, T10 | Only `lint`, `effects` or `deny` ran under the denial, never the whole gate | Closed: step 3's offline `just check` |
| T10 | Network denial on Linux (`unshare -rn`) | Open: T18 item 7 |
| T00, T07, T10 | Initialization with no cache reused (T00:500-505; T07:302-303; T10:677) | Open: T18 item 3; step 3's clone reused the host's toolchain and caches |
| T06 | `just initialize` and the allium download not run; the checksum checked on a scratch download (T06:118-145) | Closed: step 3's `initialize` downloaded and installed 3.6.1; the binary's SHA-256 equals T06's |
| T00, T07, T10 | The hook shim in a primary checkout refusing a bad commit (T00:428-431; T07:302-304; T10:158-168) | Closed for the shim: step 3's clone refused a malformed spec and a misspelling. Open for the maintainer's own primary checkout, whose `.git/hooks` holds samples only: T18 item 1 |
| T06 | The spec hooks never refused a commit; `analyse-specs` never ran on a module that does not parse (T06:184-203) | Closed: step 3, both hooks `Failed` on the malformed module |
| T00 | typos `--force-exclude` reports without rewriting, shown by no control (T00:353-355) | Closed: step 3, `cmp exit 0` |
| T00, T02, T07 | `snapshots-review` and `snapshots-accept`, and `snapshots-check` on a real snapshot | Open: T18 item 2, closed by the first snapshot's ticket |
| T00, T01, T02, T10 | `audit.yml` on `main`; `just audit` | Closed: run `40042032474` (`advisories ok`) |
| T07 | `audit.yml`'s weekly schedule never observed (T07:516) | Open: T18 item 4 |
| T01 | `ci.yml`'s `workflow_dispatch` never exercised | Closed: run `40042029974` |
| T00 | `just lock`, `just lock-upgrade`, `just check-links-online` never run | Open: T18 item 4 |
| T01 | No protection diff (the before state was a 404) | Closed: step 4's diff against T01's saved after-state is empty |
| T01 | Claim 5 in the four-job run came from cache (T01:383-391) | Closed: the stub run `39874411413` had the cache miss (claims table) |
| T00 | The adapters, bridges and `.claude/settings.json` byte-identical to L, checked by review only | Closed: at `46c2e33`, `diff` against L `ccdd001` exits 0 for `CLAUDE.md`, `.github/copilot-instructions.md`, `.claude/settings.json` and 26 bridges; the two `rust-change` bridges differ in the `description:` line alone (§7) |
| T05 | The fail-closed vendoring diff ran at `c3b8ed0` only (T05:267-269) | Closed: at `46c2e33`, `git archive -o` exit 0, `tar -x -f` exit 0, `diff -r` of `allium-skill-reference/` exit 0, and the seven Allium skills identical except `propagate:12`, T05's recorded delta |
| T00, T10 | An optional banned dependency is seen (T00:405-407) | Open: T18 item 5 |
| T10 | The per-ban red harness was not re-run after the last bans (T10:370-376, :822-845) | Open: T18 item 6 |
| T10 | The std inventory's completeness has no mechanical check (T10:310-316) | Open: T18 item 12 |
| T10 | Claim 2's rustqual scoping half was never shown | Closed as moot: rustqual was rejected (claims table) |
| T02 | Fmt, features, the coverage floor, `doc`, `deps-unused` and deny's licences, sources and `openssl-sys` ban never made to fire here (T02:20-24) | Open: T18 item 9 |
| T02 | `exclude-dev` hiding dev-dependencies from `[sources]` and audit, from docs only (T02:262) | Open: T18 item 11 |
| T02 | `wildcards`, `cargo_common_metadata` and the `module_name_repetitions` removal shown in scratch copies or lint dumps only (T02:151-157, :215-226) | Closed: the scratch runs are quoted in T02's notes; nothing on `main` claims more |
| T02 | The nextest `ci` profile never selected | Closed: the handbook and the file say nothing selects it; the profile's future is a maintainer decision (above) |
| T07 | The tutorial replayed before T02, T06 and T10 merged (T07:259-296) | Open: T18 item 8 |
| T07 | L's claims carried without a run: `allium-cli` 3.6.1 on crates.io, rustqual 1.8.3's counting, Cargo's dev-dependency cycles (T07:478-479, :498-500, :530-531) | Open: T18 item 10 |
| T07 | "No custom domain" inferred from `html_url` (T07:521-522) | Closed: `cname` is `null` (step 4) |
| T01 | Bootstrap steps 3 (Chromatic) and 5 (package access) skipped | Closed: neither applies; nothing here uses Chromatic or publishes a package |

T18 is `tickets/T18-verification-gaps.md`, in the README index and graph.

### Cleanliness (step 6)

At `55e38ae`:

- `rg -n -i 'stub|lands with T|arrive with T|TODO' docs AGENTS.md .agents crates` exits 0.
  Every line names T12 (the two crate READMEs) or the specifications (`testing.md:65`,
  `api.md:71`, `architecture.md:69`), or uses the word otherwise: a dev-dependency that
  arrives "with the first test that needs it" (`testing.md:20`) and one that "lands with
  the code" (`maintain-dependencies.md:74`), "never stub" (`terminology.md:35`,
  `layering.md:57-58`, `quality-philosophy.md`'s heading), the `todo` lint
  (`configuration.md`, `crates/libpachinko/clippy.toml:19`), the stubs T00 shipped and
  lanes replaced (`0004:65`), and the vendored `propagate` skill (`:165`, `:177`, `:180`,
  `:280`, byte-identical to L).
  No line says that a T00 to T10 deliverable is still to come; a second sweep,
  `rg -n '\bT(0[0-9]|10)\b'` over the same paths, `.github`, `.config`, `Justfile` and
  `scripts`, exits 1.
- `rg -n -i 'pawdoku|sudoku|wasm'` outside `tickets/`, `allium-skill-reference/` and
  `ai_tmp/` prints `AGENTS.md`'s Provenance sentence, the eleven italic "Carried from
  libpawdoku's decision" lines, `decisions/README.md:47`, the carried-from sentences in
  `agent-contract.md`, `purpose-and-scope.md`, `work-with-the-specs.md`, `maintenance.md`,
  `architecture.md` and `quality-philosophy.md`, and decision 0008's three sentences on
  the `no_std` and wasm design it rejected, which describe the library it was carried from.

## Open points

None.
