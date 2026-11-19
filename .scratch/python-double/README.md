# Tickets for a Python double of libpachinko

These six tickets build a minimal but correct Python implementation of the domain
`docs/specs/pachinko.allium` defines. It covers the pocket queue, leases, pull, ack, nack,
renew, release, the lease's end, expiry, parking and decay. The maintainer reads Python
rather than Rust and will use the double to judge whether the specification is right.
The HTTP service is out of scope, and so, for now, is using the double as a test oracle.

The tickets follow the format and the conventions of `tickets/`. Each one cites
`tickets/CONVENTIONS.md` by section and carries word for word the same shared rules and
adversarial-review deltas. The double is derived from the specification alone. It uses
the standard library only, with `unittest` rather than pytest, and runs through its own
recipe, `just python-double`, on pixi's Python 3.14, outside `just check`.

## Before PD01 starts

- **The base branch and the worktree are the maintainer's choice**, made before PD01
  starts. Each ticket's step 1 creates its worktree on its `branch:` from the base the
  maintainer names. Each later ticket's base must hold the ticket before it.
- **Commit these files first.** They are untracked, and an untracked file does not appear
  in a new worktree. Commit `.scratch/python-double/` on the chosen base before creating
  PD01's worktree.
- **The paths are a proposal.** The tickets put the double under `double/`, the package
  at `double/pachinko_double/`, and the spec notes at `double/SPEC-NOTES.md`. A rename
  applies to all six tickets.

## Index

The `status:` field in each ticket's frontmatter is authoritative; this table is a
snapshot.

| Id | Title | File | Blocked by | Status |
| --- | --- | --- | --- | --- |
| PD01 | The foundation, the invariant checker and publish | `PD01-foundation-and-publish.md` | none | todo |
| PD02 | Pull and the lease | `PD02-pull-and-the-lease.md` | PD01 | todo |
| PD03 | Ack, nack, release and the lease's end | `PD03-ack-nack-release.md` | PD02 | todo |
| PD04 | Deferral behind the held version | `PD04-deferral.md` | PD03 | todo |
| PD05 | The clock, with lapse, parking and decay | `PD05-clock-lapse-parking-decay.md` | PD04 | todo |
| PD06 | Close-out and the specification reader's guide | `PD06-close-out.md` | PD05 | todo |

## Dependencies

The series is a chain, so no two tickets run in parallel.

```text
PD01 ──> PD02 ──> PD03 ──> PD04 ──> PD05 ──> PD06
```

## What each ticket owns

Each of the 93 catalog scenarios and each of the 32 rules is owned by exactly one ticket,
in the ownership table under that ticket's Goal. PD01 owns all 35 invariants, because the
checker lands whole before any scenario is ported.

| Ticket | Rules | Invariants | Scenarios | Scenarios still pending on exit |
| --- | --- | --- | --- | --- |
| PD01 | 6 | 35 | 11 | 82 |
| PD02 | 5 | 0 | 18 | 64 |
| PD03 | 10 | 0 | 18 | 46 |
| PD04 | 4 | 0 | 12 | 34 |
| PD05 | 7 | 0 | 34 | 0 |
| PD06 | 0 | 0 | 0 | none; the pending lists are deleted |

Four scenarios moved from the series plan's first assignment under shared rule 2:

- S080 and S088 moved from PD04 to PD05. Their Rust tests loop over both lease endings,
  release and lapse, and `tick()` lands in PD05.
- S091 moved from PD03 to PD04. It defers a newer version behind two held keys, and
  deferral lands in PD04.
- S037 moved from PD02 to PD04. It publishes a higher rank of a key while that key is
  held, which is a deferral.

## Adversarial review

Every ticket runs `tickets/CONVENTIONS.md` §14 before its final commit, with Codex
(Astra, `gpt-6-astra`) as the reviewer through subagents. Separate subagents triage the
findings and apply the valid fixes. PD01 adds a checkpoint after its design and its
invariant checker, before any scenario is ported. PD06's checkpoint reviews the whole
double and its spec notes. Both of those add a `design` lens. Until T37 lands, every
ticket embeds the corrected Codex command, which closes stdin and keeps the findings
apart from `-o`'s last message.
