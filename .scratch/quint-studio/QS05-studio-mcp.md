---
id: QS05
title: "Agents connected to Quint Studio's MCP endpoint"
status: todo
depends_on: [QS04]
parallel_with: []
branch: ticket/qs05-studio-mcp
estimated_size: S
---

# QS05: Agents connected to Quint Studio's MCP endpoint

## Context

After QS04, `just studio up` runs Quint Studio's `serve` in the project's machine, with
its UI on host loopback behind a token. Studio also serves an MCP endpoint, on guest port
7001, which QS04 left unpublished. This ticket's design assumes that endpoint takes no
token and that Studio offers no option to require one, so that anything that can open a
connection to it can call its tools; QS01 claim 15 checks that. If claim 15 found that
Studio can require a credential, step 1's amendments say so and the maintainer rules at
step 4 on using it, which changes the configurations below. The maintainer ruled on
2026-11-24 (ruling 3, listed in `.scratch/quint-studio/QS01-studio-spike.md`, Context)
that wiring agents to it is in scope, in a ticket of its own. This is that ticket.

QS01 claim 10 recorded what `quint-studio connect claude` would write, without applying
it: the endpoint's URL, path and transport, and the file it edits. That command is
written for a Studio on the same machine as the agent, and it edits the agent's own
configuration, which for Claude Code may live in the home directory. This ticket decides
instead where the configuration lives, whether it is committed, and what it holds. No
file it commits holds a credential.

Read these:

- `.scratch/quint-studio/QS01-studio-spike.md`, its hand-back notes: claims 10, 13 and
  15, and "Facts for QS05".
- `.scratch/quint-studio/QS04-just-studio.md`, its design and hand-back notes.
- `docs/decisions/0022-quint-studio-machine.md` and `docs/how-to/run-quint-studio.md`.
- `scripts/studio.sh`, `tools/quint-studio/Smolfile`.
- `AGENTS.md`, `.claude/settings.json`, and `.gitignore`'s `.claude/` block.
- Claude Code's documentation on MCP server scopes and on project configuration in
  `.mcp.json`, read on the day: which scopes exist, which file each writes, and whether
  `.mcp.json` expands environment variables.
- `tickets/CONVENTIONS.md` §11, §13 and §14.

## Goal

While `just studio up` runs, a Claude Code session started in the worktree connects to
Studio's MCP endpoint on host loopback and lists its tools, with no credential committed
and no file outside the worktree written without the maintainer's leave. Decision 0022
records the risk of an endpoint with no token.

The demo: a Claude Code session in the worktree lists Studio's MCP tools.

## Design proposed in this ticket

These are proposals. Step 1 amends them where QS01's or QS04's hand-back disagrees, and
the maintainer rules at step 4, with checkpoint 1's findings in hand.

- **The endpoint is published on host loopback only**, at
  `127.0.0.1:${STUDIO_MCP_PORT:-7001}`, with the validation and the collision check QS04
  gives `STUDIO_UI_PORT`. `just studio up` and `status` print the endpoint's URL beside
  the UI's. The MCP port, its publication and, with configuration 2, the `.mcp.json`
  mask join the machine's input digest QS04 defined, so `up` refuses a machine built
  before this ticket, naming `just studio down` and then `just studio up`, rather than
  starting it without them.
- **The configuration**, one of three, which the maintainer picks at step 4:
  1. A committed `.mcp.json` at the repository root naming one server, `quint-studio`,
     with the URL and transport QS01 recorded, its port written
     `${STUDIO_MCP_PORT:-7001}` if Claude Code expands variables there. It holds no
     credential, because the endpoint takes none. Each contributor approves it in Claude
     Code on first use. This is the proposal, if the expansion works and QS01 found no
     credential in what `connect claude` writes.
  2. A gitignored `.mcp.json`, written by `just studio up` from the port in use and
     removed by `just studio down`. Nothing is committed, at the cost of a file the
     recipe writes into the worktree. Claude Code loads that file and no diff shows it,
     so the guest must not be able to write it: `scripts/studio.sh` adds a read-only
     single-file mask over `.mcp.json`, beside QS04's, the way QS01 claim 13 found works.
     If claim 13 found a single file cannot be masked, this configuration is not
     offered.
  3. Local scope, through `claude mcp add --scope local`, which writes the home
     directory's Claude Code configuration. Every such write is outside the worktree and
     needs the maintainer's leave each time, so the recipes never do it; the page says
     how a contributor does it by hand.
- **No other agent harness here.** Codex's MCP configuration is in Open points.
- **The no-token risk** is recorded in decision 0022's Consequences: any process on the
  host that can reach `127.0.0.1` can call Studio's tools while the machine runs,
  another worktree's agent among them, and the tools can read and, if QS01 found so,
  write the worktree. Whether a browser page can reach the endpoint depends on claim 2
  below, and 0022 records its outcome. The mitigation the design keeps is time: the
  endpoint exists only between `just studio up` and `stop`.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 1 | The endpoint answers on `127.0.0.1` only | step 5, `lsof` and `curl` |
| 2 | The endpoint refuses a request whose `Origin` or `Host` header names a foreign site, so a web page in the host browser cannot drive it | step 5, `curl` with each header |
| 3 | Claude Code expands `${STUDIO_MCP_PORT:-7001}` in `.mcp.json` | step 6, with `STUDIO_MCP_PORT` set to another port |
| 4 | Claude Code in the worktree lists Studio's tools | step 6 |
| 5 | Nothing outside the worktree changes, beyond Claude Code's own record of the maintainer's approval, in any location QS01 step 3's probe can read. Locations it cannot read are listed as unprobed, and the claim stays open for them | step 6, QS01 step 3's probe with its controls |
| 6 | With configuration 2, a write to `.mcp.json` from the guest fails and the host's copy is unchanged | step 6, one write attempt |

## Non-goals

- No token for the endpoint, if QS01 claim 15 found Studio offers none; adding a proxy
  in front of it is Open points. If claim 15 found one, the step 4 ruling decides.
- No Codex or other harness wiring.
- No MCP server committed for any tool but Studio.
- No change to the agent contract's permissions. `AGENTS.md` gains a pointer, nothing
  more.
- No `CHANGELOG.md` entry.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review" (T37 is
still todo). One delta overrides them: the reviewer is Codex Sol, `-m gpt-6.2-sol`, in
place of the `-m gpt-6-astra` that AP01's command and §14 print, by the maintainer's
choice of 2026-11-25. AP01's fallback to a fresh-context `claude-opus-5-5` subagent
still applies, and the hand-back notes record the model that ran. `<id>` is `QS05`.
There are two checkpoints, at steps 3 and 8.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tools/quint-studio/Smolfile` | shared, own lines | port 7001 published, if the Smolfile holds the ports |
| `scripts/studio.sh` | shared, own lines | `STUDIO_MCP_PORT`, its checks, the endpoint printed; with configuration 2, writing and removing `.mcp.json`, and its read-only mask |
| `.mcp.json` | new | only with configuration 1 |
| `.gitignore` | shared, own lines | only with configuration 2: `/.mcp.json`, with a comment |
| `AGENTS.md` | shared, own lines | one sentence: Studio's MCP endpoint, when it runs, and where its configuration lives |
| `docs/decisions/0022-quint-studio-machine.md` | shared, own lines | the endpoint, the configuration chosen, and the no-token risk |
| `docs/how-to/run-quint-studio.md` | shared, own lines | a section on connecting Claude Code |
| `docs/reference/configuration.md` | shared, own lines | `STUDIO_MCP_PORT`, and `.mcp.json` if it is committed |
| `docs/explanation/security-model.md` | shared, own lines | the endpoint's reach |
| `docs/project/repository-map.md` | shared, own lines | `.mcp.json`, if it is committed |
| `.scratch/quint-studio/QS05-studio-mcp.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS05's Status cell |

## Steps

1. **Worktree, baseline and the facts.** Create the worktree on
   `ticket/qs05-studio-mcp` from a base that holds QS04, and record the base and its full
   commit id. Run `just initialize`, `just install-smolvm` and
   `just install-quint-studio` (network, authorised), and `just check` on the untouched
   worktree, and quote its last line. Read the hand-backs this ticket cites and Claude
   Code's documentation on the day, and write `ai_tmp/qs05/amendments.md`, as QS02's
   step 1 does.
2. **Read before writing.** Write `ai_tmp/qs05/plan.md`: each configuration's exact
   content, the edit to `scripts/studio.sh` against "Design proposed" as amended, and the
   edit to each page in Files touched.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qs05/plan.md` and `ai_tmp/qs05/amendments.md`, before any tracked file
   changes). Questions:
   - Can the endpoint be published beyond host loopback?
   - Does any configuration hold, or lead a contributor to commit, a credential?
   - Does any configuration write outside the worktree without leave?
   - Can the guest write a configuration file Claude Code loads and no diff shows?
   - Is the no-token risk stated so that a contributor understands who can call the
     tools?
4. **The ruling.** **Stop** for the maintainer's choice of configuration and ruling on
   "Design proposed" as amended, with the checkpoint's findings in hand, and record it.
5. **The endpoint** (network, authorised: the guest's egress). Edit `scripts/studio.sh`
   and, if needed, the Smolfile. Show, quoting each: `just studio up` prints both URLs,
   token redacted; `lsof -nP -iTCP:7001 -sTCP:LISTEN` shows `127.0.0.1` only; a taken
   port and a malformed `STUDIO_MCP_PORT` are each refused, exiting 1, naming
   `STUDIO_MCP_PORT` and booting nothing, as QS04 step 5 does for the UI's; a machine
   left from QS04's recipe, created before the edit, is refused, exiting 1 and naming
   `just studio down`, and after `down` and `up` the endpoint answers;
   an MCP `initialize` request by `curl` answers, and the same request with
   `Origin: https://example.com`, and again with `Host: example.com`, is refused, or the
   hand-back says it is not (claims 1 and 2).
6. **Claude Code** (network, authorised: the model API). Add the chosen configuration.
   Create a marker file for QS01 step 3's probe under `ai_tmp/qs05/`. **Stop for the
   maintainer** to start Claude Code in the worktree and approve the server if it asks;
   the approval is the maintainer's action, recorded in Claude Code's own configuration.
   Then show `claude mcp list` naming the server as connected, and quote that line whole,
   since Verification matches it exactly; and a session listing Studio's tools
   (claim 4). With configuration 1, repeat with `STUDIO_MCP_PORT=7101` set for both
   `just studio up` and Claude Code (claim 3). With configuration 2, try one write to
   `.mcp.json` from the guest and show it failing and the host's copy unchanged
   (claim 6). Run the probe with its controls and its unreadable directories recorded,
   name every path outside the worktree that changed, and list each location the probe
   could not read as unprobed, where claim 5 stays open (claim 5).
7. **Words.** Edit each page in Files touched. Run `just check-docs` and
   `just check-agents`.
8. **Adversarial review** (checkpoint 2: conformance and breakage, over the diff with the
   pages included, before the final commit). Questions:
   - Could Verification pass with the endpoint reachable beyond host loopback?
   - Does `just check-agents` accept the new file, or does a validator now ignore
     something it should see?
   - Do decision 0022 and the page agree on who can call the tools?
9. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed by
   the full commit id. Run `just studio down`. Set `status: done` here and in
   `.scratch/quint-studio/README.md`, write the hand-back notes, and commit the record.
   On the record commit, run `git diff --name-only HEAD~1 HEAD`, which must list only
   this ticket and the README, then `just check` and `git status --short`, and report
   their output to the maintainer with the hand-back, headed by that commit, since a
   commit cannot quote its own check. If any fails, fix the cause in a new commit and
   run all three again. **Stop** before pushing or opening a pull request: both need
   authorisation (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- The endpoint is published on `127.0.0.1` only, with its port's checks, each quoted.
- Claims 2 and 3 have their outcomes recorded, and decision 0022 records claim 2's.
- A Claude Code session in the worktree lists Studio's tools, quoted.
- With configuration 2, the guest cannot write `.mcp.json`, quoted.
- No committed file holds a credential, and every path outside the worktree that
  changed is named.
- `just check-docs` and `just check-agents` pass.
- Both checkpoints are recorded as §14's Record asks, with AP01's deltas.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record
  commit, as step 9 reports.

## Verification

Run from a file as one `sh` script after the commit, with the machine down; `set -eu`
stops it at the first command that fails. No pipeline carries a result: each command
writes to a file, its exit status is checked, and the file is matched exactly.

```sh
set -eu
git rev-parse HEAD
connected='<the line claude mcp list printed for the server, connected, quoted at step 6>'
out=ai_tmp/qs05/verify
mkdir -p "$out"

just studio up >| "$out/up.log" 2>&1
port=${STUDIO_MCP_PORT:-7001}
lsof -nP -iTCP:"$port" -sTCP:LISTEN >| "$out/lsof.txt"
grep -F "127.0.0.1:$port (LISTEN)" "$out/lsof.txt"
rc=0; grep -v -F "127.0.0.1:$port (LISTEN)" "$out/lsof.txt" >| "$out/lsof-other.txt" || rc=$?
test "$rc" -le 1
if grep -F '(LISTEN)' "$out/lsof-other.txt"; then exit 1; else test "$?" -eq 1; fi
claude mcp list >| "$out/mcp-list.txt" 2>&1
grep -Fx "$connected" "$out/mcp-list.txt"
just studio down

just check-agents
just check
status=$(git status --short)
test -z "$status"
```

`connected` is filled in from step 6's quote before the script runs; a line naming the
server as disconnected or failed does not match it. `claude mcp list` and the server
name follow the configuration the ruling chose; step 1 corrects them if Claude Code's
output differs on the day. The first `lsof` `grep` is the positive control for the
silent probe after it, and `grep -v` failing with status 2 stops the script. If the
script stops early, run `just studio down` before running it again. Steps 5 and 6 hold
the header probes, the refusals and the session listing the tools, whose output is
quoted there.

## Open points

- Codex's MCP configuration for the same endpoint.
- A token in front of the endpoint, for instance a loopback proxy in the guest that
  checks one, if claim 2 fails or the maintainer wants it closed to other local
  processes.

## Hand-back notes

Empty until the ticket runs. Sections, in order: Base and branch; Baseline; The facts and
the amendments; Ruling; Claims; What landed; Refusals and probes; The maintainer's
checks; Checkpoint 1 record; Checkpoint 2 record; Departures; Verification; Open points.
