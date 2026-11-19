---
id: T37
title: "Review and ticket hygiene: §14's Codex command, triage at size S, a shell lint over ticket fences, and short shas in typos"
status: done
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
maintainer settled it as decision A, below.

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
a file other than the findings, whose path the brief names. §14 lets the executing agent
triage a size-S ticket's findings inline but sends any rejection to a separate triage
subagent, and every review leaves a `verdicts.md` the hand-back cites. A prek hook lints
the shell fences of every ticket not `done`, and of `CONVENTIONS.md` and `README.md`, in
the dialect each fence's tag names, using the shellcheck the existing hook pins.
`just check` fails a fence that masks a return in a command
substitution, and every fence in scope passes. Short commit ids pass typos without a word
list, and real misspellings still fail. The handbook's hook table names the new hook.

## Non-goals

- No change under `crates/`, `loadgen/` or `docs/specs/`.
- No new dependency: standard-library Python and the pinned interpreter, and the
  shellcheck that the existing hook already pins (decision 0007; `AGENTS.md`, invariant
  9). If that pin cannot be reused, the agent stops rather than add a second one (step 3).
- No rewrite of a `done` ticket's quoted output. A done ticket's commands are a record of
  what ran.
- No pipeline-masking lint. It stays in §14's breakage lens.
- No Python linter for `scripts/*.py`. It is a new dependency, which is not worth adding
  for one script.

## The decisions

The maintainer decided all three on 2026-11-17, before the ticket was picked up, each
time taking the recommended option: A (c), B (ii) and C (α). The options weighed are kept
below as the record. Only C carries a condition left for the day (step 3).

**A. Triage at size S** (item 2). Decided: (c).

| Option | Rule |
| --- | --- |
| (a) Keep §14 as written | Separate triage subagents at every size. The Record bullet (line 929) and the hand-back cite `verdicts.md`, so a review that skipped triage shows. |
| (b) Scale it | At size S the executing agent may triage and apply inline. It still writes `verdicts.md`, and the hand-back cites it. |
| **(c) Scale it, except rejections (decided)** | As (b), except that any finding the executing agent would mark invalid goes to a separate triage subagent. An author who rejects a finding is the case §14's independence exists for, and accepting one costs nothing to audit. |

**B. Which fences the hook lints** (item 3). Decided: (ii).

| Option | Scope | Cost |
| --- | --- | --- |
| (i) Every `sh` fence | All 40 | The nine failing fences in done tickets are edited, and their quoted output no longer matches the command above it. |
| **(ii) Every fence except in `done` tickets (decided)** | Tickets whose frontmatter `status:` is not `done`, plus `CONVENTIONS.md` and `README.md` | The three template fences are rewritten. A ticket leaves the lint when it is closed, and its commands are frozen with it. |
| (iii) SC2312 only, every fence | All 40, `--include=SC2312` | Narrowest, but it still fails four done tickets, so it needs (ii)'s exclusion anyway. |

A fence's tag picks the dialect: `sh` is checked with `-s sh` and `bash` with `-s bash`.
A bash fence in a ticket still in scope is retagged `bash`. The template fences replace `<id>`-style placeholders with shell variables (`"$id"`,
`"$branch"`, `"$lens"`). A variable that is never assigned fails SC2154, so each template
fence opens with example assignments (`id=T05 n=1 lens=conformance …`), and the prose
under it says what each variable holds. Probed on a scratch file: §14's rewritten command
and the two README commands fail with SC2154 bare and pass with the assignments.

**C. Where the hook gets shellcheck.** Decided: (α), on the condition step 3 checks.

| Option | How | Cost |
| --- | --- | --- |
| **(α) A second hook under the shellcheck-py repository (decided)** | Same `repo:` and `rev:`, an `alias`, and `entry` overridden to run the extractor in that hook's Python environment, where `shellcheck` is on `PATH` | One pin. Step 3 confirms that prek honours an `entry` override on a remote hook, and stops if it does not. |
| (β) A local hook, `language: python` | `additional_dependencies: [shellcheck-py==0.11.0.1]` | A second pin of the same tool, kept in step with the `rev:` by hand or by Renovate |
| (γ) shellcheck in the pixi environment | `pixi run --frozen`, as the other script hooks do | A new dependency in `pyproject.toml` and `pixi.lock`, with the decision-0007 note |

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tickets/CONVENTIONS.md` | shared, own lines | §14: the command block (lines 882 to 887) and the sentence after it; Phase 2, Phase 3 and Record (lines 911 to 932), for decision A |
| `tickets/README.md` | shared, own lines | The two fences in "How to pick up a ticket" (lines 208 to 216); T37's Status cell |
| `scripts/shell_fences.py` | new | Standard-library Python. It reads the Markdown files it is given, skips those out of scope, writes each `sh` or `bash` fence to a temporary file with the list indent removed, runs shellcheck on each, and names the file and line of each failing fence |
| `.pre-commit-config.yaml` | shared, own lines | The `shell-fences` hook, on `^tickets/.*\.md$`, under the shellcheck-py `repo:` and `rev:` (decision C) |
| `_typos.toml` | shared, own lines | Lines 24 to 25 replaced by `[default] extend-ignore-re`, with a comment |
| `docs/reference/quality-gates.md` | shared, own lines | The hook table (lines 212 to 225): a `shell-fences` row, and "and commit ids" in the `typos` row |
| `tickets/T*.md` with status not `done` | shared, own lines | Only fences that fail: retagged or rewritten. T36's fence passes today. |
| `tickets/T37-review-command-and-ticket-shell.md` | ticket | `status:` and hand-back notes |

Every scratch break below is reverted before the next step and never committed.

## Steps

1. Create the worktree on `branch:`, from `main` once T35 has reached it, or from the base
   the maintainer names. Record the base in the hand-back. Run `just initialize` there
   (network, authorised by this step). Run `just check` on the untouched worktree and
   quote its last line, headed by its commit, as the baseline.
2. **The plan.** Use the `plan-change` skill. Confirm the facts in Context at the new
   base: rerun the fence baseline and the typos probe, and record any count that moved.
   "The decisions" are settled; if a moved fact undermines one, **stop** and put it to
   the maintainer rather than re-decide it.
3. **Red first.** Make each scratch break below and quote the gate's result before
   changing anything.
   - Put a seven-character commit id that typos reads as a misspelling into one line of
     this ticket, for example the output of
     `git rev-parse --short=7 be7caf3e03bfd8a4dd360470af150a697663e094`. Quote
     `prek run typos --files <this ticket>` failing on it. Keep the break.
   - Add a fence tagged `sh` to this ticket holding `set -eu` and a `test -z` over a
     command substitution of `false`. Quote `just lint` passing, which is the gap. Keep
     the break.
   - Confirm on the day that prek runs a remote hook with an overridden `entry`
     (decision C). If it does not, **stop** and put (β) and (γ) to the maintainer.
4. **The command.** Rewrite §14's block in the form below, with the brief naming
   `findings-"$lens".md` as the file the reviewer writes, and the fence opening with
   example assignments (decision B):
   `codex exec -m gpt-6-astra -s read-only -C "$worktree" -o ai_tmp/review/"$id"-"$n"/last-"$lens".md "$brief" < /dev/null >| ai_tmp/review/"$id"-"$n"/log-"$lens".txt 2>&1`.
   Say in the prose that `-o` holds only the last message, that the log is what recovers a
   lost findings file, and that `>|` is for shells with `noclobber`. Leave the `claude -p`
   line's redirection to the findings file as it is, since its stdout is the findings.
   Confirm `codex exec --help` on the day, as §14 already asks.
5. **Triage.** Edit Phase 2, Phase 3 and Record for decision A: at size S the executing
   agent may triage and apply inline, any finding it would mark invalid goes to a separate
   triage subagent, and `verdicts.md` is written at every size and cited by the Record.
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
   - The same SC2312 fence added to a `done` ticket passes the hook, which shows the
     exclusion. Revert it.
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
- §14 states decision A's triage rule, and the Record bullet cites `verdicts.md`.
- The step 3 and step 7 probes are quoted with their output and exit status: the short
  id failing typos before step 6 and passing after it; the SC2312 fence passing
  `just lint` before the hook and failing it after; the dialect probe; the exclusion
  probe; the clean tree's fence count.
- Every fence in decision B's scope passes the hook.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.
- The hand-back records whether step 3 confirmed decision C's condition, with the probe.

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

### Base, branch and baseline

- **Base.** `origin/main` (`611a86e`) does not hold T35, so the base is `wave-2-tickets` at
  `8f27860`, which has T35 `done` and `a482609` as an ancestor. Diffs and the §14 briefs
  use `origin/wave-2-tickets...HEAD`.
- **Branch.** The work is on `tickets/t37-review-command-and-ticket-shell`, the branch the
  maintainer created, not this ticket's `branch:` value, `ticket/t37-review-and-ticket-shell`.
  It was not renamed. The maintainer keeps or renames it before the push.
- **Baseline**, at `8f27860`. `just initialize` exited 0; this is a secondary worktree, so
  it skipped `install-hooks`. `just check` exited 0, with the last line
  `All checks passed and the worktree is unchanged.`

### Step 2, the facts that moved (at `8f27860`)

- There are 42 fences tagged `sh`, not 40. The README fences moved to lines 215 and 221.
- Decision B's scope is T36, T37, T38, `CONVENTIONS.md` and `README.md`. The pinned
  shellcheck 0.11.0 fails:
  - `CONVENTIONS.md:882` (SC2261);
  - `README.md:215` (SC1009, SC1072, SC1073);
  - `README.md:221` (SC2261);
  - one fence the table in Context does not list, `T38:416`, which fails SC3040
    (`set -o pipefail` under `sh`) and passes once retagged `bash`.

  T36 and T37 pass. No moved fact undermines a decision.

### Step 3, red first (at `8f27860` plus the scratch break)

The fence break went in before the typos break: with the typos break present, `just lint`
fails, so it could not show the gap.

- **The fence gap.** A fence tagged `sh` holding `set -eu` and `test -z "$(false)"` was
  added to this section.
  - The pinned shellcheck, run as `--norc -s sh -o check-extra-masked-returns`, reports
    `SC2312 (info)` and exits 1.
  - `just lint` exits 0. That is the gap.
- **typos.** `git rev-parse --short=7 be7caf3e03bfd8a4dd360470af150a697663e094` prints
  `be7caf3`. With that id on a line of this ticket,
  `prek run typos --files tickets/T37-review-command-and-ticket-shell.md` fails. The hook
  reads the letters inside the id as a misspelt "calf", at `:263:21`, with
  `exit code: 2`, and prek exits 1.
- **Decision C's condition is confirmed.** The probe was a scratch second hook under the
  shellcheck-py `repo:` and `rev:`, carrying `alias: shell-fences-probe`,
  `entry: python ai_tmp/t37/probe_env.py`, `language: python`,
  `files: ^tickets/.*\.md$` and `types: [markdown]`.
  - `prek run --all-files shell-fences-probe` exits 0.
  - It prints `executable: …/.cache/prek/hooks/python-tEyY1hFF5MAVnVkl5ZEE/bin/python`,
    `version: 3.15.0`, `shellcheck: …/python-tEyY1hFF5MAVnVkl5ZEE/bin/shellcheck`, the
    worktree as `cwd`, and `argc: 35`, which is one invocation over the ticket Markdown.
  - So prek honours an `entry` override on a remote hook, and the override runs in that
    hook's environment with the pinned shellcheck on `PATH`.
  - Without `--verbose`, prek prints only `Passed`. With `verbose: true` on the hook, it
    prints the hook's output, so the hook carries `verbose: true`.

  The probe was reverted.

### Steps 4 to 8

- **Step 4.** `codex exec --help` (`codex-cli 0.169.1`) lists `-s, --sandbox` with
  `read-only`, `-C, --cd` and `-o, --output-last-message`. The §14 block now opens with
  example assignments, runs `mkdir -p`, then runs the ticket's Codex line exactly, then the
  `claude -p` line with its `>` kept.
- **Step 5.** §14 now says the following, and the introduction points at the size-S
  exception:
  - **Phase 2:** at size S the executing agent may triage inline, and any finding it would
    reject goes to a separate subagent whose verdict stands. `verdicts.md` is written at
    every size.
  - **Phase 3:** the executing agent may apply the fixes at size S.
  - **Record:** each checkpoint cites its `verdicts.md`.
- **Step 6, at `8f27860` plus the working tree.**
  - The step 3 id now passes typos: `Passed`, exit 0.
  - A line holding the id and a real misspelling of "receive" fails on the misspelling
    only, with `exit code: 2`. It was reverted.
  - Deleting the `ede` word failed typos in four files that name the word as a code span
    (T10, T11, `CONVENTIONS.md` twice, `docs/reference/configuration.md`), but not on
    `097ede1` itself. That word is now a second `extend-ignore-re` pattern, the code span
    alone, and a bare one fails again (probe at `b2151ed`'s content: `exit code: 2` on
    the bare word only).
- **Step 7.** `scripts/shell_fences.py` and the `shell-fences` hook are in.
  - T38's Verification fence is retagged `bash`.
  - The README fences use `id` and `branch`.
  - The probes, at `8f27860` plus the working tree, each quoting prek's own exit status:
    - The step 3 break fails `prek run --all-files shell-fences` with
      `tickets/T37-review-command-and-ticket-shell.md:265:12: note: … [SC2312]`.
      `just lint` exits 1 on it, with `shell fences in tickets…Failed`. It was reverted.
    - `set -o pipefail` in an `sh` fence fails with `…:270:8: warning: In POSIX sh, set
      option pipefail is undefined. [SC3040]`, prek exit 1. Retagged `bash`, it passes
      with prek exit 0.
    - The same SC2312 fence appended to T35 prints
      `tickets/T35-decide-q1-q2.md: skipped: status done`, prek exit 0.
    - A ticket with no fence prints `no sh or bash fence; checked nothing`, prek exit 0.
    - An unclosed fence, a `shell` tag, an untagged fence and a block-quoted fence each
      fail with prek exit 1.
    - With `shellcheck` off `PATH`, the script exits 2.
- **Step 8.** The `shell-fences` row sits above `shellcheck`, so the prose below the
  table, "the `shellcheck` hook one row up" from `actionlint`, still holds. The `typos`
  row reads "excluding the lockfiles and commit ids". `just check-docs` exits 0 with
  `Validated 48 pages and 49 canonical topics.`

### Step 9, the adversarial review (checkpoint 1, at `dac0b1b`)

- **Reviewers.** Codex `gpt-6-astra` (`codex-cli 0.169.1`), one per lens, ran in parallel
  with §14's command as rewritten, `-s read-only`. Both runs exited 0.
  - Each log printed `Reading additional input from stdin...` and then went on, growing
    past its header. §14 now says so.
  - Each reviewer wrote `findings-$lens.md` itself (7.1 KB and 6.5 KB), and `-o` put only
    the closing message in `last-$lens.md`.
  - **Can a read-only reviewer write the findings file?** Observed with `codex-cli 0.169.1`
    on this machine and its Codex configuration: yes. Each log shows a plain `cat >` from
    the reviewer's shell under `sandbox: read-only`, with no escalation. Elsewhere it may
    not, and then the log is the fallback §14 already names.
  - The `code-review` skill also ran over the diff, at medium effort.
- **Findings.** Conformance had 3 (1 medium, 2 low), breakage had 3 (3 medium), and
  code-review had 10. None was high, so no second round ran.
- **Triage** followed decision A, recorded in `ai_tmp/review/T37-1/verdicts.md`. The
  executing agent accepted 13. It would have rejected 3, which went to a fresh Claude
  subagent (`verdicts-subagent.md`), whose verdict stands. That subagent's brief was not
  wholly free of the agent's reasoning, as §14 now asks: it pointed at step 4 for CR3 and
  asked whether CR9 was a correctness problem at all. It still upheld CR3 against the
  agent. Its verdicts:
  - **CR1**, "a read-only Codex cannot write the findings": invalid. The real run wrote
    both files.
  - **CR3**, "the `claude -p` line has no stdin or stderr redirection and uses `>`":
    valid, but step 4 orders that line left as it is, so it is handed back below.
  - **CR9**, "one shellcheck process per fence": invalid. It is performance only, and the
    hook takes 0.249 s.
- **Applied at `b2151ed`.**
  - The extractor reads openers behind list markers (B1).
  - A closer counts only within three columns of the body's indent (CR5).
  - A status counts as done only when it is exactly `done`, `"done"` or `'done'` (B2).
  - Any tag that is neither `sh`, `bash` nor in `NOT_SHELL` fails (B3, CR4).
  - `check_file` returns its skip flag rather than leaving it to be parsed from text
    (CR10).
  - `ede` is now a code-span pattern (C2).
  - The §14 introduction (C3) and the `claude -p` prose (CR2) were corrected.
  - The handbook row now names the new failures and how the hook is selected (CR7, CR8).
- **Handed back.** C1, the empty hand-back, is this section. CR3 and CR6 are in Open
  points.
- **Re-run at `b2151ed`.**
  - `python3 ai_tmp/t37/probes.py` exits 0 with `probe failures: 0` over 25 cases: every
    probe above, plus a list-marker opener, an ordered-list opener, a list-marker fence
    left unclosed, `status: '''done'''`, two statuses, a tab-indented fence, a
    deep-indented closer inside a `markdown` fence, and the `bash-session` and
    `shell-script` tags.
  - `prek run --all-files shell-fences` exits 0 with
    `checked 6 fences in 5 files; skipped 30 with status done`.

### Step 10

- `git fetch origin` ran. `git merge origin/wave-2-tickets` and `git merge origin/main`
  each printed `Already up to date.` (`8f27860` and `611a86e` are ancestors), so the head
  stayed at `b2151ed`.
- With this ticket `done`, the hook skips it, so the clean-tree count drops by one fence
  and one file. The Verification quote below is taken at the commit that sets `done`.
- **Verification, at `3e0758b`.** The fence ran as one `sh -x` script with
  `.pixi/envs/default/bin` on `PATH`. This excerpt leaves out the trace of `before=`, which
  holds the whole of the old `CONVENTIONS.md`, and the body of `just check`:

  ```text
  + rg -q -- '-o ai_tmp/review/[^ ]*findings'
  + rg -q -- '-o ai_tmp/review/[^ ]*findings' tickets/CONVENTIONS.md
  + test 1 -eq 1
  + rg -n dev/null tickets/CONVENTIONS.md
  889:    < /dev/null >| ai_tmp/review/"$id"-"$n"/log-"$lens".txt 2>&1
  900:  - **Codex.** `< /dev/null` closes stdin: without it a backgrounded `codex exec` prints
  + prek run --all-files shell-fences
  shell fences in tickets..................................................Passed
    checked 5 fences in 4 files; skipped 31 with status done
  + prek run --all-files typos
  typos....................................................................Passed
  + just check
  All checks passed and the worktree is unchanged.
  + status=
  + test -z ''
  verification exit 0
  ```

## Open points

- The typos pattern exempts all-hex words of seven letters or more. If one hides a real
  misspelling, a pattern that needs a digit,
  `\\b[0-9a-f]*[0-9][0-9a-f]*\\b`, behaved the same on the scratch probe in Context.
- **Handed back (CR3).** §14's `claude -p` line has no `< /dev/null`, no stderr capture,
  and a plain `>`, which fails with "file exists" under `noclobber` when a failed round is
  rerun at the same path. Step 4 ordered that line left as it is. A follow-up could use
  `< /dev/null >| …/findings-"$lens".md 2>| …/log-claude-"$lens".txt`, after confirming
  with `claude --help` whether a backgrounded `claude -p` waits on stdin. The same
  follow-up should finish CR2. §14's prose says the Claude brief asks for the findings as
  the reply, but the fence passes one `$brief`, which names the findings path, to both
  commands.
- **Handed back (CR6).** `docs/reference/configuration.md:114`, which owns `_typos.toml`,
  still describes it as excluded paths and the words `mis`, `als` and `ede`. It names
  neither `extend-ignore-re` pattern, nor that the commit-id pattern exempts every all-hex
  word of seven letters or more. That page is outside Files touched.
