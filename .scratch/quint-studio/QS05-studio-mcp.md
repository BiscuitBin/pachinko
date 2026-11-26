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
its UI on host loopback behind a token, on a host port the recipe searches for (ruling
8, in the README of these tickets, `.scratch/quint-studio/README.md`). Studio also
serves an MCP endpoint, which QS04 left alone. The maintainer ruled on 2026-11-24
(ruling 3, in the README of these tickets) that wiring agents to it is in scope, in a
ticket of its own. This is that ticket.

This ticket was first drafted for an endpoint on a fixed guest port, which
`just studio up` would publish beside the UI's. QS01 found otherwise. Its claim 15 and
its "Facts for QS05" record where the endpoint listens in the guest, that the place is
not fixed, and why the option that publishes the UI's port cannot publish it. Facts for
QS05, under "The route that works", also records a route QS01 ran that needs no
published port: an MCP server that the agent starts as a command on the host and talks
to over standard input and output, where the command runs Studio's own MCP command
inside the running machine. The design below proposes that route. The particulars are
in those two places, and step 1 reads them there; this ticket repeats none of them.

Claim 15 also recorded that the endpoint, as QS01 started it, answered a request that
carried no credential, and that no option QS01 read names one. It says that this is the
build as observed, not a proof that nothing can require one. The design below assumes
no credential. If step 1 finds that Studio can require one, its amendments say so and
the maintainer rules at step 4 on using it, which changes the configurations below.

QS01 claim 10 recorded what `quint-studio connect claude` tried to do, run in the guest
against a stand-in for Claude Code and never on the host: the kind and scope of the
server it tried to register, and the files Studio had written, in the repository and
in the home directory, by the time it failed. That command is written for a Studio on
the same machine as the agent, and Facts for QS05 says why what it registers cannot
work from the host as written. This ticket does not run it. It decides instead where
the configuration lives, whether it is committed, and what it holds. No file it commits
holds a credential.

Three more rulings of the README bind here. Under ruling 2 as amended, the worktree is
mounted read-only, with a few writable subdirectories, so the guest cannot write a file
at the worktree's root. Under ruling 5, the machine's state lives outside the worktree,
in the state directory that QS02 defines and reaches through the state link, and
decision 0022 names both; the launcher below uses that state and no other. Under ruling
8, `just studio up` records the UI's host port and prints it on a line of its own. This
ticket reads neither that record nor that line: it reuses QS04's way of recording and
printing a port only if it publishes a port of its own.

Read these:

- `.scratch/quint-studio/README.md`, "The maintainer's rulings": ruling 2 as amended,
  and rulings 3, 5 and 8.
- `.scratch/quint-studio/QS01-studio-spike.md`, its hand-back notes: claims 10, 11 and
  15, "Facts for QS05", what "Facts for QS02" says of the environment and the working
  directory every smolvm command runs with, and "Rulings after the hand-back".
- `.scratch/quint-studio/QS02-pinned-smolvm.md`, its design and hand-back notes: the
  state directory and the state link; the launcher that `just smolvm` runs, with its
  guard, its sentinel, what it writes to each stream and the environment it gives
  smolvm; and the lock, how long the launcher holds it, and what QS02's ruling said of
  a command that runs for a long time.
- `.scratch/quint-studio/QS04-just-studio.md`, its design and hand-back notes: the
  machine's identity, its input digest, the port rule, how `scripts/studio.sh` calls
  QS02's launcher, what `up`, `status`, `stop` and `down` do and print, and each path
  outside the worktree that they write.
- `docs/decisions/0022-quint-studio-machine.md` and `docs/how-to/run-quint-studio.md`.
- `scripts/studio.sh`, `tools/quint-studio/Smolfile`, and the `smolvm` and `studio`
  recipes in the `Justfile`.
- `AGENTS.md`, `.claude/settings.json`, and `.gitignore`'s `.claude/` block.
- Claude Code's documentation on MCP server scopes and on project configuration in
  `.mcp.json`, read on the day (network): which scopes exist, which file each writes,
  where the approval of a project's server is recorded, how a server given as a command
  is started (its working directory and its environment), whether `.mcp.json` expands
  environment variables, and what `claude mcp list` does to a server it lists.
- `tickets/CONVENTIONS.md` §11, §13 and §14.

## Goal

While `just studio up` runs, a Claude Code session started in the worktree connects to
Studio's MCP server and lists its tools, by the one route step 4 selects. As proposed,
that route is a launcher the project provides: no port is published for it, and when
the machine or `serve` is not running the launcher refuses, exiting non-zero. If step 4
selects the published port instead, the endpoint answers on host loopback only, on a
recorded port of its own, and nothing listens there once the machine is stopped. On
either route no credential and no port is committed, and no file outside the worktree
is written without the maintainer's leave, beyond the state decision 0022 names.
Decision 0022 records the route and who can call the tools.

The demo: a Claude Code session in the worktree lists Studio's MCP tools.

## Design proposed in this ticket

These are proposals. Step 1 amends them where QS01's, QS02's or QS04's hand-back
disagrees, and the maintainer rules at step 4, with checkpoint 1's findings in hand.
One route is built, the launcher or the published port, never both. The first five
bullets are the launcher's, the sixth is the published port's, and the rest hold on
either route.

- **The route is a launcher, and no port is published.** `scripts/studio.sh` gains an
  action, proposed as `mcp` and run as `just studio mcp`; step 4's first ruling can
  change the name. It is the command of an MCP server on standard input and output. It
  runs, inside this worktree's machine, the command that Facts for QS05 records under
  "The route that works". It finds the machine as `up` does, from QS04's machine
  identity, so that it reaches this worktree's machine and no other. It creates, boots
  and changes no machine, and it reads no port, the UI's included.
- **Into smolvm through QS02's launcher, called directly.** Every smolvm command the
  action runs, its checks and the MCP command alike, goes through the launcher that
  QS02 built and `just smolvm` runs, called the way QS04's script calls it for `up`. It
  never goes through the `just smolvm` recipe, whose line `just` echoes. So QS02's
  guard, its lock and its controlled environment hold for the action, with the working
  directory Facts for QS02 names, and neither the state directory nor the state link
  is made or checked by any code of this ticket's. By QS02's design that launcher
  writes nothing of its own on standard output, and prints its sentinel and its
  refusals on standard error. Step 1 confirms each from QS02's hand-back; if the
  launcher as landed writes anything of its own on standard output, the amendments
  say so and the plan says how the action keeps it out of the protocol. A refusal by
  QS02's guard is passed on as it is, with its message and its exit status.
- **What each stream carries.** The launcher's command is the command the
  configuration gives Claude Code, proposed as `just studio mcp`. Every check of the
  launcher in this ticket runs that command exactly as the configuration has it, with
  its standard output and its standard error captured apart. Its standard input and
  output carry the protocol, so nothing reaches standard output but what the MCP
  command writes there: nothing from the script, the recipe, `just` or QS02's
  launcher. Every message of the script's own goes to standard error. `just` echoes a
  recipe's line before it runs it, unless the line is quieted, and prints a message of
  its own when a recipe fails. Step 1 records the stream each of the two goes to under
  the pinned `just`, and the plan says how the `studio` recipe keeps both out of the
  launcher's command, or proposes a command that does not pass through `just`. The
  action captures the standard error of the smolvm commands it runs for its checks and
  does not pass it on. So QS02's sentinel reaches the launcher's standard error once in
  a session, when the MCP command is started, and never in a refusal.
- **The launcher's refusals.** It refuses when the machine does not exist, when the
  machine is not running, and when `serve` is not running in it. A refusal comes before
  the MCP command is started, and starts nothing. It does not wait for its standard
  input to close: it exits non-zero within a bound the plan names. It writes nothing
  to standard output, and its complete standard error is one line that names
  `just studio up`.
- **The launcher and QS02's lock.** QS02's launcher holds QS02's lock until smolvm's
  run has ended, and the MCP command runs for as long as its session. Whether a command
  that runs so long may hold the lock for all of it is a question QS02's Open points
  leave to QS02's ruling, and this ticket does not settle it. Step 1 reads what was
  ruled. A lock held for a whole session would make every other operation on this
  worktree's machine wait or fail while Claude Code is connected: `just studio status`,
  `stop` and `down`, and a second session. Open points puts that to step 4.
- **A published port is an alternative only.** Publishing the endpoint on host loopback
  is not the proposal. QS01 ran no way of doing it: claim 15 and Facts for QS05 say why
  the option QS04 uses for the UI cannot, and name what is documented and was not
  tried. Three things come before anything is built on it, in this order. Step 4's
  first ruling gives or withholds leave to try it, and selects nothing. Then a
  transport proof is run by hand, in which an MCP `initialize` sent from the host is
  answered through a published port, with that port on host loopback only. Then step
  4's second ruling selects the route, with the proof in hand. If the published port is
  selected, the `mcp` action is not built, and the port follows QS04's port rule (ruling
  8) with a record of its own, beside the machine's state. With `STUDIO_MCP_PORT` unset
  the port is searched for, confirmed after the boot and recorded, as QS04 does for the
  UI's. With it set, the value is validated and refused if taken, never searched
  around. This ticket still reads nothing of the UI's port. What it reuses of QS04's is
  the convention: `up` and `status` print the MCP port on a line of its own, whose
  fixed text neither holds the fixed text of the UI's port line nor is held in it, so
  that a reader of either line finds one line. The publication and a port set by hand
  join the input digest. No default port number is written anywhere. Nothing listens
  on the recorded port after `just studio stop` or `just studio down`.
- **The machine's contract.** Facts for QS05 says what the launcher's route needs of
  the machine, and the proposal assumes QS04's machine already gives it. If step 1
  finds that the route needs anything more, step 2 stops for a ruling before its
  proof, and what is ruled in joins the machine's input digest QS04 defined. The
  published port, if it is selected, joins it as the bullet above says. Either way
  `up` then refuses a machine built before this ticket, naming `just studio down` and
  then `just studio up`, rather than starting it without the change, and step 5 shows
  that refusal on a machine the predecessor recipe made.
- **The configuration**, one of three. Step 4 picks one that goes with the route it
  selects: 1 or 3 with the launcher, 2 or 3 with the published port.
  1. A committed `.mcp.json` at the repository root naming one server, `quint-studio`,
     whose command is the launcher's. It holds no port and no credential. It goes with
     the launcher only, since a committed file that held a port would be true of one
     contributor's machine. Each contributor approves it in Claude Code on first use;
     Claude Code records the approval in a file of its own, which step 1 names, and no
     recipe makes it. This is the proposal.
  2. A gitignored `.mcp.json`, written by `just studio up` from the recorded port and
     removed by `just studio down`. It goes with the published port only. Nothing is
     committed, at the cost of a file the recipe writes into the worktree. `up` refuses
     a `.mcp.json` it did not write, and writes its own atomically, without following a
     link. `down` removes only the file `up` wrote, and only if it is unchanged.
  3. Local scope, through `claude mcp add --scope local`, which writes the home
     directory's Claude Code configuration. It goes with either route. Every such write
     is outside the worktree and needs the maintainer's leave each time, so the recipes
     never do it; the page says how a contributor does it by hand, for the route the
     ruling chose. In this ticket the one such write is step 6's, behind a stop of its
     own.

  Claude Code loads `.mcp.json`, and with configuration 2 no diff shows it, so the guest
  must not be able to write it. No mask is needed for that: under ruling 2 as amended
  the worktree's root is read-only to the guest, and claim 6 shows it for this file.
- **The UI's token is not this ticket's, and no file holds it.** `up` and `status`
  print QS04's host URL, which carries the token. Nothing in this ticket needs that
  URL. Its steps and its Verification send the output of `up` and `status` to the
  terminal or to `/dev/null`, never to a file, under `ai_tmp/` included, and a line
  quoted from either has the token redacted before it is written anywhere.
- **No other agent harness here.** Codex's MCP configuration is in Open points.
- **Who can call the tools** is recorded in decision 0022's Consequences, and it
  follows the route. With the launcher nothing listens on the host for MCP: whoever can
  run the recipe in the worktree can call Studio's tools while the machine runs, and
  the tools can read and write what the machine's mounts let Studio read and write. If
  the published port is selected, any process on the host that can reach the port can
  call them, another worktree's agent among them; whether a page in the host's browser
  can depends on claim 9 below, and 0022 records what was observed. The mitigation the
  design keeps is time: the launcher, or the port, answers only between
  `just studio up` and `stop`.

## Claims this ticket checks

Step 4 selects one route. Claims 1 to 3 are the launcher's and claims 8 to 11 the
published port's, and the claims of the route not selected are recorded as "not
applicable". Claims 4 to 6 hold on either route. Claim 7 is the transport proof's, and
is recorded as "not applicable" if step 4's first ruling gives no leave for the proof.
Claims 6 and 10 each name the configurations they hold for, and are recorded as "not
applicable" under any other.

| # | Claim | Check |
| --- | --- | --- |
| 1 | Launcher only: with the machine up, an MCP `initialize` and a tool listing each answer through the launcher's command, run as the configuration has it, each answer carrying the id of its request; standard output holds nothing but protocol messages, and standard error holds QS02's sentinel | step 5, `ai_tmp/qs05/mcp-probe.sh` in its session mode |
| 2 | Launcher only: the launcher refuses when the machine is absent, when it is stopped, and when `serve` is not running in it. With its standard input held open and a valid request on it, it exits non-zero within the bound, writes nothing to standard output, and its complete standard error is the one line naming `just studio up`, without QS02's sentinel. Nothing is started | step 5, the script in its refusal mode, in each state |
| 3 | Launcher only: the route publishes no port: the machine's published ports are the same before and after a session, and are QS04's | step 5 |
| 4 | Claude Code in the worktree lists Studio's tools | step 6 |
| 5 | No change outside the worktree survives to the probe, beyond Claude Code's own records and the state paths decision 0022 names, in any location QS01 step 3's probe can read, over each of two brackets: step 5's, from before its first boot to after its last `just studio down`, and step 6's, which holds Claude Code's actions alone. A file made and removed inside a bracket is not seen, and the claim does not cover it. Nor does it cover step 2's boot, a transport proof, step 6's own `up` and `down`, or Verification's run. Locations the probe cannot read or enumerate, the errors of its root controls included, are listed as unprobed, and the claim stays open for them | steps 5 and 6, QS01 step 3's probe with its controls |
| 6 | With configuration 1 or 2, a write to `.mcp.json` from the guest fails, because the worktree is read-only to it, and the host's copy is unchanged | step 6, one write attempt |
| 7 | Transport proof only: an MCP `initialize` sent from the host is answered through a published port, and that port listens on host loopback only | step 4, the transport proof |
| 8 | Published port only: the endpoint answers on host loopback only, matched as QS04's check matches the UI's, on a port that follows QS04's rule | step 5, `lsof` and `curl` |
| 9 | Published port only: what the endpoint answers to a request whose `Origin` or `Host` header names a foreign site, recorded as observed, with nothing more claimed about browsers | step 5, `curl` with each header |
| 10 | Published port with configuration 2 only: `up` writes `.mcp.json` and `down` removes it when it is unchanged; `up` refuses a `.mcp.json` it did not write and a symlink at that path, and handles a stale file of its own as the plan says; and `down` leaves a file that was changed after `up` wrote it | step 5, each case from a machine that is down and a fixture made for it |
| 11 | Published port only: with the machine up, an MCP `initialize` and a tool listing each answer through the recorded port, each answer carrying the id of its request; the machine's published ports are QS04's and that one; and nothing listens on the recorded port after `just studio stop`, or after `just studio down` | step 5, the script in its session mode, and `lsof` |

## Non-goals

- No token for the endpoint and no proxy in front of it; that is Open points. If step 1
  finds that Studio can require a credential, step 4's first ruling decides.
- No published port for the endpoint, unless step 4's second ruling selects it after
  the transport proof.
- No run of `quint-studio connect claude` on the host or in the machine, and none of
  the files QS01 claim 10 lists; Open points has the question.
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
There are two checkpoints, at steps 3 and 8. Checkpoint 1's brief names the requests
the builder makes of the maintainer after it: leave for the transport proof, the route,
and the local-scope registration of step 6. So no other review is needed before those
stops.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `scripts/studio.sh` | shared, own lines | on the launcher's route, the `mcp` action: the launcher and its refusals. On the published port's route: `STUDIO_MCP_PORT`, its rule, the port's record and its line, and, with configuration 2, writing and removing `.mcp.json`. On either route, only if the ruling changes the machine's contract: what joins the input digest |
| `Justfile` | shared, own lines | the `studio` recipe's comment, and its line or its attributes only where the plan needs them changed to keep `just`'s own output out of the launcher's command |
| `.mcp.json` | new | only with configuration 1: one server, whose command is the launcher's |
| `tools/quint-studio/Smolfile`, and QS04's further files under `tools/quint-studio/` | shared, own lines | only if the ruling changes the machine's contract, by something the launcher's route needs of the machine or by the published port, and then only the file that holds what changes |
| `.gitignore` | shared, own lines | only with configuration 2: `/.mcp.json`, with a comment |
| `AGENTS.md` | shared, own lines | one sentence: Studio's MCP server, when it answers, and where its configuration lives |
| `docs/decisions/0022-quint-studio-machine.md` | shared, own lines | the route, the configuration chosen, and who can call the tools |
| `docs/how-to/run-quint-studio.md` | shared, own lines | a section on connecting Claude Code |
| `docs/reference/commands.md` | shared, own lines | in the `studio` row: `mcp` on the launcher's route, the MCP port line on the published port's |
| `docs/reference/configuration.md` | shared, own lines | `.mcp.json` if it is committed, and `STUDIO_MCP_PORT` only if the published port is selected |
| `docs/explanation/security-model.md` | shared, own lines | the route's reach |
| `docs/project/repository-map.md` | shared, own lines | `.mcp.json`, if it is committed |
| `.scratch/quint-studio/QS05-studio-mcp.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS05's Status cell |

## Steps

Every quoted URL has its token replaced by `<redacted>`. The output of `just studio up`
and of `just studio status` is never sent to a file, under `ai_tmp/` included.

1. **Worktree, baseline and the facts** (network, authorised: the three installs, and
   reading Claude Code's documentation). Create the worktree on
   `ticket/qs05-studio-mcp` from a base that holds QS04, and record the base and its
   full commit id. Check that the base also holds QS01's hand-back notes, with their
   "Rulings after the hand-back" section, and the README of these tickets with rulings
   1 to 10; if either is missing, **stop** and report. Run `just initialize`,
   `just install-smolvm` and `just install-quint-studio`, and `just check` on the
   untouched worktree, and quote its last line. Read the hand-backs this ticket cites
   and Claude Code's documentation on the day, and write `ai_tmp/qs05/amendments.md`,
   as QS02's step 1 does. It holds, each with its source:
   - the route's particulars from Facts for QS05, and whether the route needs anything
     of the machine that QS04's machine does not give;
   - the state, the machine identity and the lock as QS02 and QS04 landed them, and
     what QS02's ruling said of a command that holds the lock for a long time;
   - how `scripts/studio.sh` calls QS02's launcher, what that launcher writes to each
     stream, and its sentinel;
   - the stream to which the pinned `just` echoes a recipe's line and prints its
     message for a failed recipe, and what it offers to quiet each;
   - each path outside the worktree that `just studio up`, `stop` and `down` write for
     a worktree, from decision 0022 and QS04's hand-back;
   - whether QS04's hand-back left a machine;
   - what the documentation says of how Claude Code starts a server given as a
     command, and which file records the approval of a project's server and which a
     local-scope registration writes.
2. **Read before writing, and the route by hand** (network, authorised: the image
   pull, the package snapshot and the guest's egress, since the first `up` in a new
   worktree makes the machine; writes outside the worktree: what `just studio up`
   writes for this worktree as decision 0022 and QS04's hand-back name it, which is
   its state directory and state link, the data directory, the locks, and the empty
   directory if QS04's ruling placed it outside; nothing else). Before the first `up`,
   **stop for the maintainer**: quote those paths from the amendments and ask for leave
   to use them. The leave covers every later `up`, `stop` and `down` of this ticket in
   this worktree, Verification's included. QS01 ran the route with the worktree mounted
   read-write, and never against QS04's machine. So, before the plan is built on it,
   one of two proofs runs:
   - If the amendments name nothing that the route needs beyond QS04's machine: run
     `just studio up`, and by hand through `just smolvm` send an MCP `initialize` and a
     tool-listing request by the route, quoting the command and both answers; then run
     `just studio down`.
   - If they name something more: **stop** for the maintainer's ruling on a proof
     against a disposable machine that has it. The request says what QS04's machine
     lacks, the commands that make the disposable machine by hand through
     `just smolvm`, in the form `up` uses and under a name that is not the one `up`
     uses, its mounts, what it writes outside the worktree, and the commands that
     delete it. With that leave, make the machine, send the same two requests by the
     route, quote the commands and both answers, and delete the machine, showing it
     gone from smolvm's own listing.

   If either request goes unanswered, **stop** and report, with what the route appears
   to lack where that can be told. The builder does not choose another route, and the
   maintainer may then rule the disposable proof. Then write `ai_tmp/qs05/plan.md`,
   against "Design proposed" as amended. It covers both routes, since both of step 4's
   rulings rest on it:
   - for the launcher: its exact command as the configuration has it; each of its
     checks; each refusal, with its complete standard error and the bound within which
     it exits; what reaches each stream, and how `just`'s own output is kept out; and
     the outline of step 5's probe script;
   - for the published port: the transport proof's commands by hand, the way of
     publishing it tries, what it writes outside the worktree, what it reaches on the
     network and how it is undone; then the port rule, the record, the port line and
     the handling of `.mcp.json` as they would be built;
   - each configuration's exact content, with the route it goes with, and for
     configuration 3 the exact command and the file it writes;
   - what changes in the machine's contract, if anything;
   - an answer, or the question as it stands, for each question under Open points;
   - the edit to each page in Files touched, for each route.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qs05/plan.md` and `ai_tmp/qs05/amendments.md`, before any tracked file
   changes). The brief names the three requests "Adversarial review" lists. Questions:
   - Can the launcher reach a machine other than this worktree's, or create, boot or
     change one?
   - Can the launcher start with the machine or `serve` down and not refuse, so that
     Claude Code holds a server that answers nothing?
   - Can a refusal pass its check after the MCP command was started, or because the
     launcher is broken or missing?
   - Does anything but the protocol reach the launcher's standard output, from the
     script, the recipe, `just` or QS02's launcher?
   - Does the route open a port on the host, or is one published beyond host loopback?
   - Does the transport proof, as planned, publish beyond host loopback, write outside
     the worktree what its request does not name, or leave anything behind?
   - Does any configuration hold, or lead a contributor to commit, a credential, a
     port or a path that is true of one contributor's machine only?
   - Does any configuration write outside the worktree without leave?
   - Can the guest write a configuration file Claude Code loads and no diff shows?
   - Is it stated who can call the tools, so that a contributor understands it?
4. **The rulings.** There are two, in this order, each recorded as it was given.
   - **The first ruling.** **Stop** for the maintainer's ruling on "Design proposed" as
     amended, the launcher's name and each question under Open points, with the
     checkpoint's findings in hand. The same stop asks whether the maintainer gives
     leave for the transport proof, and quotes the plan's commands for it, what it
     writes outside the worktree and what it reaches on the network. This ruling
     selects no published port. If it gives no leave, the route is the launcher, the
     ruling picks configuration 1 or 3, claims 7 to 11 are recorded as "not
     applicable", and step 5 follows.
   - **The transport proof**, only with that leave (network, authorised by the first
     ruling: the image pull, the package snapshot and the guest's egress; writes
     outside the worktree: what step 2's leave covers and what the first ruling named,
     nothing else). Run it by hand as the plan has it. An MCP `initialize` sent from
     the host is answered through a published port, and `lsof` on that port shows
     listeners on host loopback only, matched as QS04's check matches the UI's (claim
     7). Quote the commands, their output and their exit statuses to the maintainer.
     Then undo it, and show that nothing of it is left: no machine of the proof's in
     smolvm's own listing, no listener on the port by `lsof`, and `just studio status`
     printing the line it gives for no machine.
   - **The second ruling**, only after a transport proof. **Stop** again, with the
     proof quoted. The maintainer selects the route, the launcher or the published
     port, and a configuration that goes with it: 1 or 3 with the launcher, 2 or 3 with
     the published port. A proof that failed leaves the launcher as the only route. The
     claims of the route not selected are recorded as "not applicable".
5. **The route** (network, authorised: the image pull, the package snapshot and the
   guest's egress, on every `up` that makes a machine; writes outside the worktree:
   what step 2's leave covers, nothing else). In this order, quoting each run with its
   exit status:
   - Create a marker file for QS01 step 3's probe under `ai_tmp/qs05/`, before any
     boot of this step. It opens the bracket that the probe closes in the last bullet.
   - Only if the design as ruled changes the machine's contract: before any edit, make
     a machine with the predecessor recipe, `scripts/studio.sh` as the base commit has
     it, since QS04's hand-back leaves none running, and leave it for the refusal
     below.
   - Edit `scripts/studio.sh` and, only as the ruling needs them, the `Justfile` and
     the files that hold the machine's contract. Write `ai_tmp/qs05/mcp-probe.sh`,
     which is never committed and uses only what macOS and the pixi environment
     provide. It takes a mode and a directory, and first removes the files it writes in
     that directory, so that nothing left by an earlier run can pass.

     In its session mode it runs the launcher's command, exactly as the configuration
     has it, with files or descriptors of its own on the command's standard input,
     output and error. It sends an `initialize` and a tool-listing request, each with
     an id made for this run. It writes `stdout.txt` and `stderr.txt`, the command's
     two streams apart and whole; `initialize.txt`, one line naming the server as its
     answer does; and `tools.txt`, one tool's name a line. The last two are taken only
     from answers that carry this run's ids. It exits 0 only if both requests were so
     answered and standard output held nothing but protocol messages.

     In its refusal mode it writes a valid `initialize` request to the command's
     standard input and holds that input open. It waits for the command to exit, for
     the plan's bound at most, and writes `stdout.txt`, `stderr.txt` and `status.txt`,
     which holds the command's exit status. It exits 0 only if the command exited of
     itself, with a status that is not zero, within the bound and with its input still
     open; otherwise it ends the command and fails.

     On the published port's route there is no refusal mode, and the session mode
     takes the recorded port as a third argument and sends the same two requests
     through it.
   - If the predecessor machine was made: `just studio up` refuses it, exiting 1 and
     naming `just studio down`; then run `just studio down`.
   - **On the launcher's route** (claims 1 to 3):
     - `just studio up`, then the script in its session mode: its exit status and its
       four files, with `initialize.txt` and `tools.txt` quoted whole, since
       Verification matches a line of each exactly (claim 1). `stderr.txt` holds QS02's
       sentinel on a line of its own. Quote that line whole: Verification matches it,
       and it is the positive control for the refusals.
     - The machine's published ports before the script's session and after it, the
       same both times and those QS04's hand-back records (claim 3).
     - The refusals, in three states: with `serve` stopped inside the running machine,
       by hand through `just smolvm`; after `just studio stop`; and after
       `just studio down`. In each, the script in its refusal mode exits 0,
       `status.txt` holds a status that is not zero, `stdout.txt` is empty, and
       `stderr.txt` is the one line and nothing else, so it holds no sentinel. Nothing
       was started: `just studio status` reads as it did before the refusal, and in
       the first state a process listing of the guest, taken by hand through
       `just smolvm`, shows neither `serve` nor the MCP command (claim 2). Quote the
       `stderr.txt` of the run after `just studio down` whole, since Verification
       compares the whole file.
   - **On the published port's route** (claims 8 to 11):
     - `just studio up`: `up` and `status` print the MCP port on its own line, the
       same port both times, and the machine's published ports are QS04's and that
       one. The script in its session mode, given that port: its exit status, and
       `initialize.txt` and `tools.txt` quoted whole (claim 11).
     - `lsof` on that port shows host loopback only, matched as QS04's check matches
       the UI's. A taken port and a malformed `STUDIO_MCP_PORT` are each refused,
       exiting 1, naming `STUDIO_MCP_PORT` and booting nothing, and the search and the
       collision are shown, as QS04 shows them for the UI's (claim 8).
     - An MCP `initialize` request by `curl` answers; and the same request with
       `Origin: https://example.com`, and again with `Host: example.com`, has its
       answer recorded as observed (claim 9).
     - After `just studio stop`, and again after `just studio down`, `lsof` finds no
       listener on that port. Its exit status and its empty output are quoted, and the
       listener seen while the machine ran is the positive control (claim 11).
     - With configuration 2 only (claim 10), each case from a machine that is down, no
       `.mcp.json` in the worktree, and a fixture made for that case and removed after
       it. Every refusal exits 1, boots nothing, and leaves the fixture as it was
       (`cmp` against a copy under `ai_tmp/qs05/`).
       - The control: `up` writes `.mcp.json`, quoted; `down` removes it, shown with
         `test ! -e` and `test ! -L`.
       - A regular `.mcp.json` that `up` did not write: `up` refuses.
       - A symlink at `.mcp.json` to a file under `ai_tmp/qs05/`: `up` refuses, and
         the link and its target are unchanged.
       - A stale file of `up`'s own, made by copying the file aside after an `up` and
         putting it back after the `down`: `up` does what the plan says.
       - A file changed after `up` wrote it: `down` leaves it, as changed, and says
         so; what `down` then does to the machine is the plan's. Remove the file by
         hand and run `just studio down` again.
   - Last, with the machine down after this step's last `just studio down`: the probe
     against the marker, with its controls and its unreadable directories recorded.
     Name every path outside the worktree that changed. Beyond the state paths
     decision 0022 names, and Claude Code's own records if the builder is itself a
     Claude Code session, there may be none. List as unprobed each location the probe
     could not read or enumerate, the errors of its root controls included; claim 5
     stays open there. The probe sees only what survives to it. So where QS02's
     launcher, as landed, runs smolvm inside a write boundary like the one section 9
     of QS01's hand-back describes, cite that boundary as the stronger evidence for
     what ran under it, and say what did not run under it (claim 5).
6. **Claude Code** (network, authorised: the image pull, the package snapshot and the
   guest's egress for the `up`, and the model API for Claude Code; writes outside the
   worktree: what step 2's leave covers, and Claude Code's own records, which the
   maintainer's actions make; with configuration 3, the home directory's Claude Code
   configuration, behind the stop below; nothing else). Add the chosen configuration:
   with configuration 1, the committed `.mcp.json`; with configuration 2 nothing,
   since `up` writes the file; with configuration 3 nothing in the worktree. Run
   `just studio up`, then create a second marker file under `ai_tmp/qs05/`, so that
   this bracket holds Claude Code's actions and no start or stop of the machine. With
   configuration 3 only, and immediately before the write: **stop for the maintainer**,
   quoting the exact `claude mcp add --scope local` command and the file it writes, and
   ask the maintainer to run it or to give leave for this one run; record which, and
   the command that removes the registration. Then **stop for the maintainer** to start
   Claude Code in the worktree and approve the server if it asks; the approval is the
   maintainer's action, recorded in Claude Code's own configuration. Show that session
   listing Studio's tools (claim 4). When the session has ended, since a connected
   session may hold QS02's lock and a listing starts a launcher of its own, show
   `claude mcp list` naming the server as connected, and quote that line whole, since
   Verification matches it exactly. Then run the probe against the second marker,
   with its controls and its unreadable directories recorded, and name every path
   outside the worktree that changed: beyond Claude Code's own records and the state
   paths decision 0022 names there may be none. List as unprobed each location the
   probe could not read or enumerate, where claim 5 stays open (claim 5). Record apart,
   by path and without quoting their content, the files the approval changed and, with
   configuration 3, the files the registration changed, inside the worktree or outside
   it. `git status --short`, taken before the maintainer's actions and after them, must
   read the same; if it does not, **stop** and report, since a file Claude Code writes
   that Git can see would fail `just check`. Then, with no session connected, since a
   session may hold QS02's lock: with configuration 1 or 2, try one write to
   `.mcp.json` from the guest, by hand through `just smolvm`, and show it failing and
   the host's copy unchanged, by `cmp` against a copy under `ai_tmp/qs05/` (claim 6).
   Last, run `just studio down`: Verification starts with the machine down.
7. **Words.** Edit each page in Files touched, for the route and the configuration that
   were ruled. Run `just check-docs` and `just check-agents`.
8. **Adversarial review** (checkpoint 2: conformance and breakage, over the diff with
   the pages included, before the final commit). The README's "Rules every ticket
   holds" binds from step 1 to step 9, and here it means that the evidence a fix of
   this checkpoint touches is run again and replaced before the record commit.
   Questions:
   - Could Verification pass with the route answering from a machine other than this
     worktree's?
   - On the launcher's route: could it pass with a port published for MCP? Could its
     refusal check pass because the launcher is broken or missing, or after the MCP
     command was started, and not because the launcher refused?
   - On the published port's route: could it pass with the port published beyond host
     loopback, or still answering after `just studio down`?
   - Does any file under `ai_tmp/qs05/`, or any quote, hold the UI's token?
   - Does `just check-agents` accept the new file, or does a validator now ignore
     something it should see?
   - Do decision 0022 and the page agree on who can call the tools?
9. **Commit and hand back.** Commit the work. Rerun Verification, the script of the
   route that was ruled, and quote it, headed by the full commit id. Run
   `just studio down`. Set `status: done` here and in
   `.scratch/quint-studio/README.md`, write the hand-back notes, and commit the record.
   On the record commit, run `git diff --name-only HEAD~1 HEAD`, which must list only
   this ticket and the README, then `just check` and `git status --short`, and report
   their output to the maintainer with the hand-back, headed by that commit, since a
   commit cannot quote its own check. If any fails, fix the cause in a new commit and
   run all three again. **Stop** before pushing or opening a pull request: both need
   authorisation (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- The route answered an MCP `initialize` and a tool listing by hand, against QS04's
  machine or against the disposable machine the maintainer ruled, before the plan was
  written, quoted (step 2).
- Both of step 4's rulings are recorded as given, or the first alone if it gave no
  leave for the transport proof. If the proof ran, its commands and output are quoted,
  and nothing of it is left (step 4).
- On the launcher's route, with the machine up, an MCP `initialize` and a tool listing
  answer through the launcher's command as the configuration has it, with nothing but
  protocol on standard output and QS02's sentinel on standard error, quoted (step 5).
- On the launcher's route, the launcher refuses in each of the three states with its
  input held open: it exits non-zero within the bound, its standard output is empty,
  its complete standard error is the one line naming `just studio up`, and nothing is
  started, each quoted (step 5).
- On the launcher's route, no port is published for MCP: the machine's published ports
  before and after a session are quoted (step 5).
- On the published port's route, an MCP `initialize` and a tool listing answer through
  the recorded port; the endpoint is published on host loopback only, with its port's
  checks; nothing listens on the port after `stop` or after `down`; and claim 9's
  outcome is recorded, in decision 0022 too, each quoted (step 5).
- On the published port's route with configuration 2, claim 10's five cases are quoted,
  the control that `down` removes the unchanged file among them (step 5).
- If the ruling changed the machine's contract, `up` refuses a machine that the
  predecessor recipe made, exiting 1 and naming `just studio down`, quoted (step 5).
- A Claude Code session in the worktree lists Studio's tools, quoted (step 6).
- With configuration 1 or 2, the guest cannot write `.mcp.json`, quoted (step 6).
- No committed file holds a credential or a port, and no file under `ai_tmp/qs05/` and
  no quote holds the UI's token.
- Every path outside the worktree that survived to the probe is named, for each of the
  two brackets, with each location the probe could not read listed as unprobed; and
  the files the approval and, with configuration 3, the registration changed are named
  (steps 5 and 6).
- Decision 0022 records the route, the configuration chosen and who can call the
  tools.
- The claims of the route not selected, claim 7 if no proof ran, and claims 6 and 10
  under a configuration they do not name, are recorded as "not applicable".
- `just check-docs` and `just check-agents` pass.
- Both checkpoints are recorded as §14's Record asks, with AP01's deltas.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record
  commit, as step 9 reports.

## Verification

There are two scripts, one for each route, and the one for the route that was ruled is
run, from a file, as one `sh` script after the commit and after `just studio down`, so
with the machine down and no Claude Code session connected in this worktree, since a
connected session may hold QS02's lock; `set -eu` stops it at the first command that
fails. It is never run with `-x`, which could print the UI's token. It reaches the
network (the image, the package snapshot and the guest's egress, since its `up` makes
the machine again, and whatever `claude mcp list` reaches), under the ticket's
authorisation. It writes outside the worktree only what step 2's leave covers and
Claude Code's own records. No pipeline carries a result: each command writes to a file,
its exit status is checked, and the file is matched exactly. There is one exception to
"writes to a file", on the published port's route alone, and the text after that
script gives its reason. On both routes the standard output of `just studio up`, which
holds the UI's token, goes to `/dev/null` and reaches no file.

On the launcher's route:

```sh
set -eu
git rev-parse HEAD
sentinel='<the line QS02 launcher prints as its sentinel, quoted at step 5>'
refusal='<the whole of stderr.txt after just studio down, one line, quoted at step 5>'
server='<the line of initialize.txt, quoted at step 5>'
tool='<one line of tools.txt, quoted at step 5>'
connected='<the line claude mcp list printed for the server, connected, quoted at step 6>'
out=ai_tmp/qs05/verify
mkdir -p "$out/session" "$out/down"
printf '%s\n' "$refusal" >| "$out/refusal.expected"

# Green: a session through the launcher. Its sentinel is the positive control for
# the refusal below, whose standard error must be the one line and so hold none.
just studio up >/dev/null 2>| "$out/up.err"
sh ai_tmp/qs05/mcp-probe.sh session "$out/session"
grep -Fx "$server" "$out/session/initialize.txt"
grep -Fx "$tool" "$out/session/tools.txt"
grep -Fx "$sentinel" "$out/session/stderr.txt"
claude mcp list >| "$out/mcp-list.txt" 2>&1
grep -Fx "$connected" "$out/mcp-list.txt"
just studio down

# Red: with the machine down the launcher refuses, with its input held open.
sh ai_tmp/qs05/mcp-probe.sh refusal "$out/down"
test -s "$out/down/status.txt"
if grep -Fx 0 "$out/down/status.txt"; then exit 1; else test "$?" -eq 1; fi
test ! -s "$out/down/stdout.txt"
cmp "$out/refusal.expected" "$out/down/stderr.txt"

just check-agents
just check
status=$(git status --short)
test -z "$status"
```

The five placeholders are filled in from the quotes of steps 5 and 6 before the script
runs. A status that is not zero is not enough for the refusal, since a missing recipe
or action gives one too: the probe holds the launcher's input open with a valid request
on it, so a launcher that started the MCP command would answer or wait, not exit; the
standard output must be empty; and `cmp` takes the whole of the standard error, so one
more line, QS02's sentinel or a message of `just`'s among them, fails it. The session
comes first because its sentinel line shows that the sentinel can be seen at all. The
probe removes its own files from the directory it is given before it runs, and takes
`initialize.txt` and `tools.txt` only from answers that carry the ids it sent, so a
file left by an earlier run cannot pass. A line naming the server as disconnected or
failed does not match `connected`. Steps 5 and 6 hold the other two refusals, the
comparison of the published ports, the guest's write and the session listing the
tools, whose output is quoted there.

On the published port's route:

```sh
set -eu
unset STUDIO_MCP_PORT
git rev-parse HEAD
config='<the configuration step 4 ruled: 2 or 3>'
port_label='<the fixed text before the number on the MCP port line, quoted at step 5>'
loopback='<the loopback addresses of QS01 claim 5, as lsof prints them, spaced apart>'
server='<the line of initialize.txt, quoted at step 5>'
tool='<one line of tools.txt, quoted at step 5>'
connected='<the line claude mcp list printed for the server, connected, quoted at step 6>'
out=ai_tmp/qs05/verify
mkdir -p "$out/session"

just studio up >/dev/null 2>| "$out/up.err"

# The one command substitution. status prints the token URL beside the MCP port line,
# so its output is held in a variable, never in a file, and only the port is kept.
printed=$(just studio status 2>| "$out/status.err")
test -n "$printed"
port=
nl='
'
old_ifs=$IFS
set -f
IFS=$nl
for line in $printed; do
    case "$line" in
    "$port_label"*)
        test -z "$port"
        port=${line#"$port_label"}
        ;;
    esac
done
IFS=$old_ifs
set +f
unset printed line
case "$port" in
'' | *[!0-9]*) exit 1 ;;
esac
printf '%s\n' "$port" >| "$out/mcp-port.txt"

# Green: the listeners on that port are on host loopback only.
lsof -nP -iTCP:"$port" -sTCP:LISTEN >| "$out/lsof.txt"
: >| "$out/allowed.txt"
set -f
for addr in $loopback; do
    printf '%s:%s (LISTEN)\n' "$addr" "$port" >> "$out/allowed.txt"
    grep -F "$addr:$port (LISTEN)" "$out/lsof.txt"
done
set +f
test -s "$out/allowed.txt"
rc=0; grep -v -F -f "$out/allowed.txt" "$out/lsof.txt" >| "$out/lsof-other.txt" || rc=$?
test "$rc" -le 1
if grep -F '(LISTEN)' "$out/lsof-other.txt"; then exit 1; else test "$?" -eq 1; fi

# Green: the exchange through the recorded port, and Claude Code connected.
sh ai_tmp/qs05/mcp-probe.sh session "$out/session" "$port"
grep -Fx "$server" "$out/session/initialize.txt"
grep -Fx "$tool" "$out/session/tools.txt"
if test "$config" = 2; then
    test -f .mcp.json
    test ! -L .mcp.json
fi
claude mcp list >| "$out/mcp-list.txt" 2>&1
grep -Fx "$connected" "$out/mcp-list.txt"
just studio down

# Red: after down nothing listens on the port, and the generated file is gone.
rc=0
lsof -nP -iTCP:"$port" -sTCP:LISTEN >| "$out/lsof-down.txt" 2>| "$out/lsof-down.err" ||
    rc=$?
test "$rc" -eq 1
test ! -s "$out/lsof-down.txt"
test ! -s "$out/lsof-down.err"
if test "$config" = 2; then
    test ! -e .mcp.json
    test ! -L .mcp.json
fi

just check-agents
just check
status=$(git status --short)
test -z "$status"
```

The six placeholders are filled in before the script runs: `config` from step 4's
second ruling, `port_label`, `server` and `tool` from step 5's quotes, `connected` from
step 6's, and `loopback` from QS01's hand-back. The assignment to `printed` is the one
exception to the rule that every result goes through a file. `status` prints the host
URL, with the UI's token, beside the MCP port line, and no file may hold the token. So
the output is read once into a variable, by one command substitution whose exit status
`set -e` checks; `test -n` stops an empty read; the loop keeps the number of the one
line that begins with `port_label` and stops at a second such line; and the variable
is unset. `printed` is never echoed, and no here-document is used, which a shell may
back with a temporary file. The loop splits on newlines alone and runs under `set -f`,
so a line is never taken as a file pattern. The script unsets `STUDIO_MCP_PORT`, so the
port is a searched one.

The loopback check is the one QS04's Verification runs for the UI's port, here on the
MCP port: each `grep` inside the loop is a positive control for the silent probe after
it, `test -s` stops a `loopback` left empty, and `grep -v` failing with status 2 stops
the script. Step 1 corrects the check from QS04's hand-back if QS04 landed it
otherwise, and the `lsof` patterns if the line shape QS01 claim 5 quoted differs. The
`lsof` that finds a listener while the machine runs is the positive control for the one
after `down`, which must exit 1 with nothing on either stream: that is what `lsof` gives
when it finds no listener, which step 5 shows, and any other failure stops the script.
Steps 5 and 6 hold the port's refusals, the search and the collision, the header
probes, the run after `stop`, claim 10's cases, the guest's write and the session
listing the tools, whose output is quoted there.

For both scripts: `ai_tmp/qs05/mcp-probe.sh` is step 5's script and is not committed,
so the script runs in the worktree that ran step 5. `just studio mcp` stands, here and
in the steps, for the launcher's command as the ruling and the configuration have it.
`claude mcp list` and the server name follow the configuration the ruling chose; step 1
corrects them if Claude Code's output differs on the day. The standard error of `up`
and of `status` is kept in files, since by QS04's design it never carries the token;
step 1 confirms that from QS04's hand-back, and if it can carry the token it goes to
`/dev/null` as well. If the script stops early, run `just studio down` before running
it again.

## Open points

Questions for step 4's first ruling, which the plan of step 2 puts to the maintainer
with what step 1 found:

- How the launcher learns its worktree. A committed `.mcp.json` must be the same for
  every contributor, and the launcher must act on the worktree the session was started
  in. In which working directory, and with which environment, does Claude Code start
  the server's command, how does that command find the `Justfile`, and how much of
  that environment may reach smolvm? Does the command pass through `just` at all,
  given what `just` writes of its own?
- The launcher and the lock. By QS02's design its launcher holds the lock until
  smolvm's run has ended, so a launcher that goes through it holds the lock for its
  whole session. Whether a command that runs so long may do that is QS02's Open point,
  and QS02's ruling settles it, not this ticket. Step 1 reads the ruling. If it left
  the hold as it is, what do `just studio status`, `stop` and `down` do while a session
  is connected, and does the maintainer accept that? If it gave long-running commands
  another way in, the launcher uses that way and the plan says how.
- Two sessions at once. Two Claude Code sessions in one worktree each start a
  launcher. Are both served, or is the second refused? The answer follows the lock.
- A session that outlives its machine. What does the launcher do, and what does Claude
  Code show, when the machine stops or `serve` restarts under a connected session?
- State that is absent. QS02's launcher makes the state directory when it is absent,
  and the state link again after a restart of the host, for any command that goes
  through it. Does the MCP launcher rely on that, so that a refusal in a worktree with
  no state still makes both, or does it refuse first and name `just studio up`?
- A machine `up` would refuse. Does the launcher also refuse a machine whose recorded
  input digest differs from the one it computes?
- The bound within which a refusal exits, and whether the same bound is the longest
  the launcher's checks may take before the MCP command is started.
- If leave is given for the transport proof: which way of publishing it tries, of
  those Facts for QS05 names as not tried and any that step 1 finds.
- If the published port is selected with configuration 2: whether Claude Code asks for
  approval again when `up` writes `.mcp.json` anew with another port, which step 1
  reads from the documentation, and what the page tells a contributor then.

Left for later:

- Codex's MCP configuration for the same server.
- A token in front of the endpoint, for instance a proxy in the guest that checks one,
  if the maintainer wants the tools closed to other local processes, or, with a
  published port, if claim 9 shows a foreign `Origin` or `Host` is answered.
- Whether the project wants the skill and the plugin files that
  `quint-studio connect claude` writes, which QS01 claim 10 lists. This ticket carries
  none of them.

## Hand-back notes

Empty until the ticket runs. Sections, in order: Base and branch; Baseline; The facts
and the amendments; The route by hand; Rulings; The transport proof, or that none ran;
Claims; What landed; Refusals and probes; The maintainer's checks; Checkpoint 1 record;
Checkpoint 2 record; Departures; Verification; Open points.
