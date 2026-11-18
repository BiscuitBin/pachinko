---
id: T37
title: "Review and ticket hygiene: §14's Codex command, triage at size S, a shell lint over ticket fences, and short shas in typos"
status: todo
depends_on: [T35]
parallel_with: [T33, T36]
branch: ticket/t37-review-and-ticket-shell
estimated_size: S
---

# T37: Review and ticket hygiene: §14's Codex command, triage at size S, a shell lint over ticket fences, and short shas in typos

## Context

A retrospective on the session that redrew decision 0020 and wrote T36 found four defects
in how tickets are written and reviewed. None of them is in the crates. Line numbers are
as of `a482609c14d0a98887d2a74eb54dc2cd6a5a747c`, the head of `ticket/t35-decide-q1-q2`
when this ticket was written.

**1. §14 prints a Codex command that hangs and one that loses its findings.** The command
block in `tickets/CONVENTIONS.md` §14 (lines 882 to 887) runs
`codex exec … -o ai_tmp/review/<id>-<n>/findings-<lens>.md "<brief>"`.

- It does not close stdin. Run in the background, `codex exec` prints
  `Reading additional input from stdin...` and waits forever, with no error and no
  findings file. T17's second review sat idle for over an hour this way.
- `-o` writes the agent's last chat message to the path it names, after the run. When the
  brief also tells the reviewer to write its findings there, the findings are overwritten
  by a one-line message. Recovering them from the log cost two passes in the T22 to T31
  review.
- The fix so far lived only in one maintainer's local agent notes, so any other agent or
  contributor who follows §14 as printed repeats both failures. `rg -n 'dev/null'
  tickets/CONVENTIONS.md` prints nothing today.

**2. §14's triage and apply phases were skipped at size S.** §14 Phase 2 (lines 911 to
915) has separate subagents triage each finding into `ai_tmp/review/<id>-<n>/verdicts.md`.
Phase 3 (lines 916 to 918) has them apply the valid fixes one writer at a time. T36's two
review rounds, eleven and then three findings, were triaged and applied by the
executing agent itself, at commits `a9de4f2` and `a482609`, and no `verdicts.md` was
written. That is a deviation from the convention as written. For a size-S ticket that
changes only documents, the convention may also cost more than it protects. The
maintainer decides which, at step 2.

**3. Shell in tickets is never linted.** T36's second round found
`test -z "$(git status --short)"`, which passes when `git` fails, and a `git show | rg`
pipeline that hides a failed `git` the same way. §14's breakage lens (lines 897 to 903)
lists this class by hand, so it recurs.

- `tickets/*.md` holds 40 fences tagged `sh`: `rg -c '^\s*```sh' tickets/*.md`, summed.
- The `shellcheck` hook (`.pre-commit-config.yaml` lines 170 to 173, shellcheck-py
  `v0.11.0.1`, shellcheck 0.11.0) checks shell files only, so no fence reaches it.
  `shellcheck` is not on `PATH` and not in the pixi environment; it exists only in prek's
  environment for that hook.
- `shellcheck -o check-extra-masked-returns` reports the command-substitution case as
  SC2312. It does not report a pipeline whose first command fails; that stays a review
  question.
- `just lint`, gate 3 of `just check`, runs `prek run --all-files` (`Justfile` lines 294
  to 296), so a new hook in `.pre-commit-config.yaml` runs in `just check` and in CI
  without a recipe edit.

A baseline, taken by writing each `sh` fence to its own file, with a list item's
three-space indent removed, and running
`shellcheck -s sh -o check-extra-masked-returns` on each: 12 of the 40 fail.

| File | Fences failing | Codes | Status |
| --- | --- | --- | --- |
| `tickets/CONVENTIONS.md` | 1 (§14's block) | SC2261 | not a ticket |
| `tickets/README.md` | 2 ("How to pick up a ticket") | SC1009, SC1072, SC1073, SC2261 | not a ticket |
| `T18`, `T28`, `T29`, `T31` (2), `T32` (2), `T34`, `T35` | 9 | SC2312, SC3040, SC3001, SC3052, SC2034, SC2154, SC2016, SC2260 | done |

- SC2261, SC1009, SC1072 and SC1073 come from `<id>`-style placeholders, which the shell
  reads as redirections. The rewrite in item 1 adds more of them unless it uses
  variables.
- SC3040, SC3001 and SC3052 are `set -o pipefail`, process substitution and similar:
  those fences are bash tagged `sh`.
- SC2312 hits sit in T28, T31, T32 and T35, all `done`, whose hand-back notes quote the
  output of those commands.

**4. typos is patched one commit id at a time.** `_typos.toml` lines 24 to 25 add one
word so that T10's citation of `097ede1` passes. A seven-character prefix of
`be7caf3e03bfd8a4dd360470af150a697663e094` fails typos 1.51.1 today (exit 2), so T36
cites the full id, which passes. `rg -n '\b[0-9a-f]{7}\b' tickets/` matches 801 lines. In
a scratch configuration holding only
`[default] extend-ignore-re = ["\\b[0-9a-f]{7,40}\\b"]`, both short ids pass and a real
misspelling on the same line still fails. The pattern also exempts any all-hex word of
seven letters or more, such as "defaced"; a misspelling made only of the letters a to f
would slip through.

## Goal

§14 prints a Codex command that closes stdin, logs stdout and stderr, and points `-o` at
a file other than the findings, whose path the brief names. §14 says, as the maintainer
chose at step 2, who triages at size S, and every review leaves a `verdicts.md` the
hand-back cites. A prek hook lints the shell fences in `tickets/*.md`, in the scope and
dialect chosen at step 2. `just check` fails a fence that masks a return in a command
substitution, and every fence in scope passes. Short commit ids pass typos without a word
list, and real misspellings still fail. The handbook's hook table names the new hook.

## Non-goals

- No change under `crates/`, `loadgen/` or `docs/specs/`.
- No new dependency: standard-library Python and the pinned interpreter, and the
  shellcheck that the existing hook already pins, unless step 2 picks option (γ) below,
  which then needs the decision-0007 note (`AGENTS.md`, invariant 9).
- No rewrite of a `done` ticket's quoted output. A done ticket's commands are a record of
  what ran.
- No pipeline-masking lint. It stays in §14's breakage lens.
- No Python linter for `scripts/*.py`. It is a new dependency, which is not worth adding
  for one script.

## The choice for step 2

Three decisions belong to the maintainer. The agent presents them with the
recommendations and **stops** for the answers.

**A. Triage at size S** (item 2).

| Option | Rule |
| --- | --- |
| (a) Keep §14 as written | Separate triage subagents at every size. The Record bullet (line 929) and the hand-back cite `verdicts.md`, so a review that skipped triage shows. |
| (b) Scale it | At size S the executing agent may triage and apply inline. It still writes `verdicts.md`, and the hand-back cites it. |
| (c) Scale it, except rejections (recommended) | As (b), except that any finding the executing agent would mark invalid goes to a separate triage subagent. An author who rejects a finding is the case §14's independence exists for, and accepting one costs nothing to audit. |

**B. Which fences the hook lints** (item 3).

| Option | Scope | Cost |
| --- | --- | --- |
| (i) Every `sh` fence | All 40 | The nine failing fences in done tickets are edited, and their quoted output no longer matches the command above it. |
| (ii) Every fence except in `done` tickets (recommended) | Tickets whose frontmatter `status:` is not `done`, plus `CONVENTIONS.md` and `README.md` | The three template fences are rewritten. A ticket leaves the lint when it is closed, and its commands are frozen with it. |
| (iii) SC2312 only, every fence | All 40, `--include=SC2312` | Narrowest, but it still fails four done tickets, so it needs (ii)'s exclusion anyway. |

Under (i) and (ii), a fence's tag picks the dialect: `sh` is checked with `-s sh` and
`bash` with `-s bash`. A bash fence in a ticket still in scope is retagged `bash`. The
template fences replace `<id>`-style placeholders with shell variables (`"$id"`,
`"$branch"`, `"$lens"`). A variable that is never assigned fails SC2154, so each template
fence opens with example assignments (`id=T05 n=1 lens=conformance …`), and the prose
under it says what each variable holds. Probed on a scratch file: §14's rewritten command
and the two README commands fail with SC2154 bare and pass with the assignments.

**C. Where the hook gets shellcheck.**

| Option | How | Cost |
| --- | --- | --- |
| (α) A second hook under the shellcheck-py repository (recommended, if prek allows it) | Same `repo:` and `rev:`, an `alias`, and `entry` overridden to run the extractor in that hook's Python environment, where `shellcheck` is on `PATH` | One pin. Step 3 confirms that prek honours an `entry` override on a remote hook, and stops if it does not. |
| (β) A local hook, `language: python` | `additional_dependencies: [shellcheck-py==0.11.0.1]` | A second pin of the same tool, kept in step with the `rev:` by hand or by Renovate |
| (γ) shellcheck in the pixi environment | `pixi run --frozen`, as the other script hooks do | A new dependency in `pyproject.toml` and `pixi.lock`, with the decision-0007 note |

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tickets/CONVENTIONS.md` | shared, own lines | §14: the command block (lines 882 to 887) and the sentence after it; Phase 2, Phase 3 and Record (lines 911 to 932) as choice A decides |
| `tickets/README.md` | shared, own lines | The two fences in "How to pick up a ticket" (lines 208 to 216) under B (i) or (ii); T37's Status cell |
| `scripts/shell_fences.py` | new | Standard-library Python. It reads the Markdown files it is given, skips those out of scope, writes each `sh` or `bash` fence to a temporary file with the list indent removed, runs shellcheck on each, and names the file and line of each failing fence |
| `.pre-commit-config.yaml` | shared, own lines | The `shell-fences` hook, on `^tickets/.*\.md$`, as choice C decides |
| `_typos.toml` | shared, own lines | Lines 24 to 25 replaced by `[default] extend-ignore-re`, with a comment |
| `docs/reference/quality-gates.md` | shared, own lines | The hook table (lines 212 to 225): a `shell-fences` row, and "and commit ids" in the `typos` row |
| `tickets/T*.md` with status not `done` | shared, own lines | Under B (i) or (ii), only fences that fail: retagged or rewritten. T36's fence passes today. |
| `pyproject.toml`, `pixi.lock`, `docs/decisions/0007-dependency-policy.md` | shared, own lines | Under C (γ) only |
| `tickets/T37-review-command-and-ticket-shell.md` | ticket | `status:` and hand-back notes |

Every scratch break below is reverted before the next step and never committed.

## Steps

1. Create the worktree on `branch:`, from `main` once T35 has reached it, or from the base
   the maintainer names. Record the base in the hand-back. Run `just initialize` there
   (network, authorised by this step). Run `just check` on the untouched worktree and
   quote its last line, headed by its commit, as the baseline.
2. **The plan and the choice.** Use the `plan-change` skill. Confirm the facts in Context
   at the new base: rerun the fence baseline and the typos probe, and record any count
   that moved. Present "The choice for step 2" with the recommendations, A (c), B (ii)
   and C (α), and **stop** for the answers. Record them, quoted and dated, in the
   hand-back.
3. **Red first.** Make each scratch break below and quote the gate's result before
   changing anything.
   - Put a seven-character commit id that typos reads as a misspelling into one line of
     this ticket, for example the output of
     `git rev-parse --short=7 be7caf3e03bfd8a4dd360470af150a697663e094`. Quote
     `prek run typos --files <this ticket>` failing on it. Keep the break.
   - Add a fence tagged `sh` to this ticket holding `set -eu` and a `test -z` over a
     command substitution of `false`. Quote `just lint` passing, which is the gap. Keep
     the break.
   - Under C (α), confirm on the day that prek runs a remote hook with an overridden
     `entry`. If it does not, stop and present (β) and (γ) again.
4. **The command.** Rewrite §14's block in the form below, with the brief naming
   `findings-<lens>.md` (or `"$lens"` under B (ii)) as the file the reviewer writes:
   `codex exec -m gpt-6-astra -s read-only -C "$worktree" -o ai_tmp/review/"$id"-"$n"/last-"$lens".md "$brief" < /dev/null >| ai_tmp/review/"$id"-"$n"/log-"$lens".txt 2>&1`.
   Say in the prose that `-o` holds only the last message, that the log is what recovers a
   lost findings file, and that `>|` is for shells with `noclobber`. Leave the `claude -p`
   line's redirection to the findings file as it is, since its stdout is the findings.
   Confirm `codex exec --help` on the day, as §14 already asks.
5. **Triage.** Edit Phase 2, Phase 3 and Record as choice A decides.
6. **typos.** Replace lines 24 to 25 of `_typos.toml`. Quote the break from step 3 now
   passing `prek run typos`. Then quote a scratch line holding a short id and a real
   misspelling failing on the misspelling only. Revert both.
7. **The hook.** Use the `fix-quality` skill. Write `scripts/shell_fences.py` and the
   hook. Fix or retag the fences in scope that fail. Quote each probe with its output and
   exit status:
   - The break from step 3 fails the hook with SC2312 and names the file and line. Revert
     it.
   - A bash-only line (`set -o pipefail`) in a fence tagged `sh` fails. Retagged `bash`,
     it passes. Revert both.
   - Under B (ii), the same SC2312 fence added to a `done` ticket passes the hook, which
     shows the exclusion. Revert it.
   - A file with no fence passes, and the hook says it checked nothing in that file, not
     that it checked everything.
   - The clean tree: `prek run --all-files shell-fences` exits 0 and prints how many
     fences it checked.
8. **The handbook.** Use the `review-docs` skill. Edit the two rows in
   `docs/reference/quality-gates.md`. Run `just check-docs`.
9. **Adversarial review** (`tickets/CONVENTIONS.md` §14), conformance and breakage, a
   first round, and a second over the fix diff if any high-severity finding is valid. Use
   the command as step 4 rewrote it, and triage as step 5 decided. The breakage lens asks
   whether the hook can pass while a fence in scope masks a return: whether it finds every
   fence, indented ones included, whether a fence it cannot parse passes, whether an
   unknown tag is skipped silently, and whether the out-of-scope test can drop a ticket
   that is not done.
10. Commit. Run `git fetch origin` (network, authorised by this step), merge the base
    branch and `origin/main`, rerun `just check`, and rerun and requote Verification,
    each headed by its commit (§11, "Green" and "Quoted evidence names its commit"). Set
    `status: done` here and in `tickets/README.md`, and write the hand-back. **Stop**
    before pushing or opening a pull request; both need authorisation.

## Acceptance criteria

- §14's Codex command closes stdin, logs its output, and points `-o` away from the
  findings file the brief names.
- §14 states the triage rule step 2 chose, and the Record bullet cites `verdicts.md`.
- The step 3 and step 7 probes are quoted with their output and exit status: the short
  id failing typos before step 6 and passing after it; the SC2312 fence passing
  `just lint` before the hook and failing it after; the dialect probe; the exclusion
  probe under B (ii); the clean tree's fence count.
- Every fence in the chosen scope passes the hook.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.
- The step 2 choices are quoted in the hand-back with their date.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. This fence
passes `shellcheck -s sh -o check-extra-masked-returns` as written.

```sh
set -eu
old=a482609c14d0a98887d2a74eb54dc2cd6a5a747c
before=$(git show "$old":tickets/CONVENTIONS.md)
printf '%s\n' "$before" | rg -q -- '-o ai_tmp/review/[^ ]*findings'   # positive control
if rg -q -- '-o ai_tmp/review/[^ ]*findings' tickets/CONVENTIONS.md; then exit 1; else test "$?" -eq 1; fi
rg -n 'dev/null' tickets/CONVENTIONS.md                               # §14 closes stdin
prek run --all-files shell-fences                                      # prints the count
prek run --all-files typos
just check
status=$(git status --short)
test -z "$status"
```

## Hand-back notes

## Open points

- The typos pattern exempts all-hex words of seven letters or more. If one hides a real
  misspelling, a pattern that needs a digit,
  `\\b[0-9a-f]*[0-9][0-9a-f]*\\b`, behaved the same on the scratch probe in Context.
