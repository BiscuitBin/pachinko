# Tickets for Quint Studio in a project-managed smolvm machine

Quint Studio is Informal Systems' desktop application for Quint, with an embedded server
that runs headless as `quint-studio serve`. Decision 0021,
`docs/decisions/0021-project-managed-quint-cli.md`, keeps it outside the repository,
because upstream publishes it only as an unpinned `stable/latest` download with no
checksum.

Five tickets, QS01 to QS05, bring it under the project the way AP01 and AP02,
`.scratch/apalache/`, brought Apalache: pinned by a hash this repository records,
installed into the worktree, kept off the global `PATH`, and reached only through
guarded recipes. Two kinds of durable data are kept outside the worktree: the machine's
state (ruling 5), which QS02's removal recipe removes, and Studio's data with the
sign-in, for which QS04 proposes the place and says what removes it. Removing the first
is not to be read as removing the second. QS04 also proposes a lock outside any
worktree, which every worktree shares. Studio runs inside a smolvm microVM, an x86_64
Linux guest under Rosetta on the maintainer's macOS arm64 machine, with the worktree
mounted read-only, with a few writable subdirectories. QS06 is a follow-up after those
five (ruling 10). Each ticket cites `tickets/CONVENTIONS.md` by section and follows the
format of `tickets/`.

## The maintainer's rulings

Rulings 1 to 4 were taken on 2026-11-24. Ruling 2 was amended, and rulings 5 to 10 were
taken, on 2026-11-25, after QS01's hand-back. That hand-back records each answer under
"Rulings after the hand-back", numbered there in its own order, which is not the
numbering here. QS01's outcome was go. Every ticket holds these rulings and
cites this section for them: it is their full statement. QS01's Context lists rulings 1
to 4 as they were first taken, with ruling 2 before its amendment.

1. Studio is pinned by its observed SHA-256. A new upstream build fails the install
   until it is re-pinned. That detects drift; it does not make installs reproducible.
2. Amended on 2026-11-25; until then it read "The worktree is mounted read-write". The
   worktree is mounted read-only, with a few writable subdirectories, such as
   `quint-specs/`, which QS04 names. Tracked files outside them are read-only to Studio,
   `tools/`, `fuzz/` and `loadgen/` among them, which the maintainer accepts. The guest
   has open egress, and host loopback and private ranges are blocked. QS01 mounted the
   worktree read-write throughout, so the read-only mount is untested: QS02 shows that
   the mounts work, QS04 shows that Studio's `serve`, sign-in, a scan and its git
   commands work under them and which subdirectories must be writable, and either stops
   for the maintainer if they do not. Where in the guest the worktree is mounted is not
   part of the ruling: QS01 mounted it at its own host path, read-write, and QS02 and
   QS04 propose that same path for the read-only mount. That is their proposal, not the
   maintainer's ruling.
3. Wiring agents to Studio's MCP endpoint is in scope, as its own ticket.
4. The only supported host is macOS on arm64. Linux hosts are recorded as untested.
5. State. smolvm's machine state lives outside the worktree, in a durable directory of
   its own for each worktree, for example under `~/Library/Caches/pachinko/`. smolvm is
   given a short symbolic link to it in `/tmp`, which the recipe makes again after a
   restart, since macOS empties `/tmp` then. The link is there because smolvm limits the
   length of its state root as the path is written; QS01's "Facts for QS02" gives the
   limit. This is state outside the worktree, so decision 0022 records it and a recipe
   removes it. It is untested and rests on smolvm's source as read: QS01 ran no absolute
   root that fitted and no root through a symbolic link. QS02 runs both before it builds
   on them, and stops for the maintainer if either fails.
6. Hiding. The maintainer's answer was "One hidden parent", given to a question about
   the state directory before ruling 5 took the state out of the worktree. What is left
   of that answer is the lead's reading, not the maintainer's words: a project-managed
   ignored tree that the guest must not read is hidden by an empty directory mounted
   over it, and such trees share one parent, so that one mount hides them. Under ruling
   2 such a mount is no longer needed to stop writes, only to hide: a read-only mount
   stops writes, not reads. QS04 proposes the list of what is hidden, and the maintainer
   rules on it at QS04's stop for its ruling.
7. Git. The common git directory is mounted read-only. QS04 ships a wrapper around the
   guest's `git` that sends the objects Studio adds to a directory on the machine's own
   disk. It compares the common directory with the exact mounted path before it
   redirects, which the wrapper QS01 tested did not.
8. The host port. The maintainer's answer was "option 1, But to help make it easier to
   run across different work tree directories and also not conflict with other
   projects, have it search for an open port." Option 1 was a host port other than
   7000. The reason for it was given in the question, not in the answer: macOS's AirPlay
   Receiver holds host port 7000 on the maintainer's machine (QS01's Open point 6). So
   the guest's port stays Studio's own, and `just studio up` publishes it on a free host
   port that it searches for, for each worktree. The host port is never 7000, whether it
   is searched for or set by hand. `up` records the port where the printed URL reads it.
   QS05 does not read that record: it reuses QS04's way of recording a port only if it
   publishes a port of its own. QS01's hand-back names QS05's wiring as a reader of the
   record; that wording is the lead's, not the maintainer's, and these tickets do not
   follow it.
9. Not hermetic. Accepted and documented. QS01 recorded, as its Open point 7, that
   Studio has an upgrade command, checks for updates, downloads an executable into its
   data directory, queues telemetry and carries a crash-report client. The AppImage's
   pin covers none of that. Decision 0022 and the handbook say what the pin does not
   cover. Egress stays as ruling 2 has it.
10. The guest's packages. The maintainer's answer was "Got with option 1 for now, but
    create a follow up ticket for later (after mvp) to see if we can build and host it
    as an artifact on github." Option 1 was a dated snapshot: QS04 installs the guest's
    packages from a fixed `snapshot.debian.org` timestamp and install-tests it. QS06 is
    the follow-up ticket: after the first working version, it finds out whether the
    guest image can be built and hosted as an artifact on GitHub. Reading "after mvp" as
    after the first working version, QS02 to QS05, and so placing QS06 after QS05, is
    the lead's reading, not the maintainer's words.

Three further answers of 2026-11-25 are not rulings on the design, and QS01's hand-back
records them, not this section: leave to group the lines of its probe list, the
instruction to carry these rulings into these tickets, and the choice to keep QS01's
branch local.

## Before QS01 starts

- **The base must hold AP02.** QS02 onwards edit the lines AP02 added: the network
  recipes in `AGENTS.md`, `just initialize` and gate 2. AP02 landed on `quint-spec`
  (`ffe512b`), which has not merged into `main`. Until it does, every QS ticket bases on
  a branch that holds both AP02 and these files, and the maintainer names it before
  step 1. The base branch and the worktree are the maintainer's choice.
- **These files are committed on `quint-studio-smolmachine`**, which is based on
  `quint-spec`. `.scratch/` is tracked, as `.scratch/apalache/` is, so a later ticket
  reads an earlier one's hand-back from the base it is built on. It is not scratch in
  `tickets/CONVENTIONS.md` §11's sense, which is `ai_tmp/` and the session's scratch
  directory, never committed.
- **QS01 decided whether the rest runs, and the outcome is go.** It is a spike, and it
  ran on 2026-11-25. Its hand-back says go on one condition, which the amendment of
  ruling 2 meets, so with the rulings above the outcome is go and QS02 to QS05 run. The
  series can still stop: QS02 and QS04 each prove, before they build on it, what the
  rulings rest on and QS01 did not test, and a proof that fails is a stop for the
  maintainer.
- **The base must hold QS01's hand-back and these tickets as amended.** QS02 onwards run
  on a base that holds both QS01's hand-back, with its "Rulings after the hand-back"
  section, and these tickets as amended on 2026-11-25. `quint-studio-smolmachine` has
  held both since `ticket/qs01-studio-spike` was merged into it on 2026-11-25, with
  the maintainer's leave, locally and unpushed. Step 1 of each ticket still checks
  that both are there on the base it starts from.
- **The facts in QS01's Context were read from upstream documentation, not run.** No
  later ticket treats one as settled until QS01's hand-back records it. Every name,
  flag, digest and hash a later ticket uses comes from that hand-back, not from the
  ticket's text.

## Rules every ticket holds

QS02 to QS06 each hold these four rules from step 1 on, and each points here from its
final checkpoint. `tickets/CONVENTIONS.md` §11 states them in full, in the paragraphs
named; where §11 writes `main` or `origin/main`, read the base the maintainer names.

- **A red baseline is a stop.** If the baseline `just check` on the untouched worktree
  is red, the ticket stops for the maintainer and edits nothing: the fault is the
  base's, not the ticket's work ("Green").
- **Every quote names its commit.** Each quoted output is headed by the commit it ran
  at, with "with uncommitted changes" where that is so ("Quoted evidence names its
  commit").
- **A fix makes evidence stale.** After a fix made at the final checkpoint, the
  evidence the fix touches is run again and replaced before the record commit, a
  step's proof that Verification does not rerun included (the same paragraph).
- **The integration rerun comes before any request to push.** Before it asks to push
  or to open a pull request, the ticket rebases onto, or merges, the base the
  maintainer names then, reruns `just check`, and reruns and requotes its Verification
  ("Green", and "Separately authorised actions" for the request itself).

## Index

The `status:` field in each ticket's frontmatter is authoritative; this table is a
snapshot.

| Id | Title | File | Blocked by | Status |
| --- | --- | --- | --- | --- |
| QS01 | Spike: Quint Studio `serve` in a smolvm guest, by hand | `QS01-studio-spike.md` | none | done |
| QS02 | Decision 0022, a pinned smolvm, and a project-managed machine that boots | `QS02-pinned-smolvm.md` | QS01 | done |
| QS03 | Quint Studio's AppImage pinned by its observed SHA-256 | `QS03-pinned-studio.md` | QS01 | todo |
| QS04 | `just studio`: Quint Studio running in the project's machine | `QS04-just-studio.md` | QS02, QS03 | todo |
| QS05 | Agents connected to Quint Studio's MCP endpoint | `QS05-studio-mcp.md` | QS04 | todo |
| QS06 | Finding out: can the guest image be built and hosted on GitHub | `QS06-guest-image.md` | QS05 | todo |

## Dependencies

```text
QS01 ──> QS02 ──┐
   └───> QS03 ──┴──> QS04 ──> QS05 ──> QS06
```

QS01 booted Studio's `serve` in a smolvm guest by hand and recorded every name, path,
flag and hash the rest need. It ran on 2026-11-25, and with the rulings above its
outcome is go; the proofs QS02 and QS04 run first can still stop the series. QS02 pins
smolvm by its release checksum, installs it into the worktree, adds `just smolvm`, which
keeps the machine's state outside the worktree (ruling 5), adds the recipe that removes
that state, and writes decision 0022. QS03 pins the Studio AppImage by the hash the
maintainer reviewed. QS02 and QS03 can be built in parallel, but QS03 lands after QS02,
rebased onto it, because both edit the same pages and gate 2, and QS03 cites decision
0022. QS04 adds the committed Smolfile and `just studio up|status|stop|down`, with the
worktree mounted read-only, the UI on host loopback on a host port it searches for, and
a how-to page. It proposes a directory outside the worktree for Studio's data, which
holds the sign-in, and says what removes it; and it proposes a lock that every worktree
shares, so that two searches never take the same port. QS05 connects Claude Code to
Studio's MCP endpoint by the route QS01 tested, through a guarded launcher. QS06 comes
after QS05, by the lead's reading of "after mvp" as after the first working version,
QS02 to QS05 (ruling 10): it finds out whether the guest image can be built and hosted
as an artifact on GitHub, in place of the package install in the provisioning step QS04
writes.

## Out of scope for all five

The five are QS01 to QS05. QS06 names its own non-goals.

- Linux hosts. Ruling 4 records them as untested; the installers refuse them and
  `just initialize` skips them.
- A CI job that installs smolvm or boots a machine. No runner offers the hypervisor.
- Renovate tracking for smolvm or Studio. Both are moved by hand, as Apalache is, by the
  procedures QS02 and QS03 write.
- A gate that runs Studio, or a `.qnt` gate written with it.

## Adversarial review

Each ticket runs `tickets/CONVENTIONS.md` §14 with Codex (Sol, `gpt-6.2-sol`) as the
reviewer, by the maintainer's choice of 2026-11-25, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review", since
T37, which corrects §14's command, is still todo. AP01's command and §14 print
`gpt-6-astra`; each ticket states the change of model as a delta. QS01 has four
checkpoints, numbered in its "Adversarial review" section: one before each of its two
"Authorisation required" stops, run only if the stop is reached; one over the choices of
steps 3 to 5, which the later steps build on; and one over its hand-back notes before
its final commit. QS02, QS03 and QS05 each have two: over the builder's plan before any
tracked file changes, and over the diff before the final commit. QS04 has those two and
one more before them, over the leave its proof step asks for. QS06 numbers its
checkpoints in its own "Adversarial review" section: one over its plan before anything is built, one
over its hand-back notes before its final commit, and one before each separately
authorised action that is reached.
