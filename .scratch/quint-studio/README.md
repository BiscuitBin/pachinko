# Tickets for Quint Studio in a project-managed smolvm machine

Quint Studio is Informal Systems' desktop application for Quint, with an embedded server
that runs headless as `quint-studio serve`. Decision 0021,
`docs/decisions/0021-project-managed-quint-cli.md`, keeps it outside the repository,
because upstream publishes it only as an unpinned `stable/latest` download with no
checksum.

These five tickets bring it under the project the way AP01 and AP02,
`.scratch/apalache/`, brought Apalache: pinned by a hash this repository records,
installed into the worktree, never written to the home directory or the global `PATH`,
and reached only through guarded recipes. Studio runs inside a smolvm microVM, an
x86_64 Linux guest under Rosetta on the maintainer's macOS arm64 machine, with the
worktree mounted. Each ticket cites `tickets/CONVENTIONS.md` by section and follows the
format of `tickets/`.

## The maintainer's rulings

Taken on 2026-11-24. Every ticket holds them, and QS01's Context states them in full.

1. Studio is pinned by its observed SHA-256. A new upstream build fails the install
   until it is re-pinned. That detects drift; it does not make installs reproducible.
2. The worktree is mounted read-write. The guest has open egress, and host loopback and
   private ranges are blocked.
3. Wiring agents to Studio's MCP endpoint is in scope, as its own ticket.
4. The only supported host is macOS on arm64. Linux hosts are recorded as untested.

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
- **QS01 decides whether the rest runs.** It is a spike. If its hand-back says no-go,
  QS02 to QS05 do not start, and the hand-back's fallbacks go to the maintainer.
- **The facts in QS01's Context were read from upstream documentation, not run.** No
  later ticket treats one as settled until QS01's hand-back records it. Every name,
  flag, digest and hash a later ticket uses comes from that hand-back, not from the
  ticket's text.

## Index

The `status:` field in each ticket's frontmatter is authoritative; this table is a
snapshot.

| Id | Title | File | Blocked by | Status |
| --- | --- | --- | --- | --- |
| QS01 | Spike: Quint Studio `serve` in a smolvm guest, by hand | `QS01-studio-spike.md` | none | todo |
| QS02 | Decision 0022, a pinned smolvm, and a project-managed machine that boots | `QS02-pinned-smolvm.md` | QS01 | todo |
| QS03 | Quint Studio's AppImage pinned by its observed SHA-256 | `QS03-pinned-studio.md` | QS01 | todo |
| QS04 | `just studio`: Quint Studio running in the project's machine | `QS04-just-studio.md` | QS02, QS03 | todo |
| QS05 | Agents connected to Quint Studio's MCP endpoint | `QS05-studio-mcp.md` | QS04 | todo |

## Dependencies

```text
QS01 ──> QS02 ──┐
   └───> QS03 ──┴──> QS04 ──> QS05
```

QS01 boots Studio's `serve` in a smolvm guest by hand and records every name, path,
flag and hash the rest need, or a no-go. QS02 pins smolvm by its release checksum,
installs it into the worktree, adds `just smolvm`, and writes decision 0022. QS03 pins
the Studio AppImage by the hash the maintainer reviewed. QS02 and QS03 can be built in
parallel, but QS03 lands after QS02, rebased onto it, because both edit the same pages
and gate 2, and QS03 cites decision 0022. QS04 adds the committed Smolfile and
`just studio up|status|stop|down`, with the UI on host loopback, and a how-to page. QS05
publishes the MCP endpoint on host loopback and connects Claude Code to it.

## Out of scope for all five

- Linux hosts. Ruling 4 records them as untested; the installers refuse them and
  `just initialize` skips them.
- A CI job that installs smolvm or boots a machine. No runner offers the hypervisor.
- Renovate tracking for smolvm or Studio. Both are moved by hand, as Apalache is, by the
  procedures QS02 and QS03 write.
- A gate that runs Studio, or a `.qnt` gate written with it.

## Adversarial review

Each ticket runs `tickets/CONVENTIONS.md` §14 with Codex (Astra, `gpt-6-astra`) as the
reviewer, with the deltas `.scratch/apalache/AP01-java-and-verify.md` writes under
"Adversarial review", since T37, which corrects §14's command, is still todo. QS01 has
four checkpoints, numbered in its "Adversarial review" section: one before each of its two
"Authorisation required" stops, run only if the stop is reached; one over the choices
of steps 3 to 5, which the later steps build on; and one over its hand-back notes before
its final commit. QS02 to QS05 each have two: over the builder's plan before any
tracked file changes, and over the diff before the final commit.
