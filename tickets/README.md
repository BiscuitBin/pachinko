# Tickets for bootstrapping `pachinko`

This directory is the work breakdown for the foundation of `BiscuitBin/pachinko`, a Rust
workspace of a library, `libpachinko`, and a service built around it, `pachinko`. The
domain is not defined yet: Allium specifications come next, then the implementation.
These tickets carry libpawdoku's toolchain, gates, handbook, agent contract, CI and
Allium tooling across, so that the specifications and the code land in a repository that
is already green.

Each ticket is written for an AI agent with no other context, working in its own git
worktree. `CONVENTIONS.md` is the shared design every ticket obeys and cites by section;
read it first, then the ticket.

Pushing, opening pull requests, tagging, filing these as GitHub issues, changing
repository or organisation settings, installing a GitHub App and editing any other
repository are separately authorised actions. Nothing here has been filed.

## Index

The `status:` field in each ticket's frontmatter is authoritative; this table is a
snapshot.

| Id | Title | File | Depends on | Parallel with | Status |
| --- | --- | --- | --- | --- | --- |
| T00 | Foundation: a shape-complete skeleton that passes `just check` on its first commit | `T00-foundation.md` | none | none | open |
| T01 | GitHub repository and CI: settings, branch protection, Pages, workflows | `T01-github-and-ci.md` | T00 | T02, T05, T06, T07, T10 | open |
| T02 | Rust quality gate: final tool configs, the std lint table, every Rust recipe green on real code | `T02-rust-gate.md` | T00 | T01, T05, T06, T07 | open |
| T05 | Agent contract: AGENTS.md, fourteen skills, the Allium vendoring | `T05-agent-contract.md` | T00 | T01, T02, T06, T07, T10 | open |
| T06 | Allium gate: the placeholder module, the spec recipes, the two spec pages | `T06-allium-gate.md` | T00 | T01, T02, T05, T07, T10 | open |
| T07 | Handbook: every page, and decision records 0001 to 0014 | `T07-handbook.md` | T00 | T01, T02, T05, T06, T10 | open |
| T10 | Metrics and the effects boundary: rustqual over both crates, the libpachinko bans, the probe | `T10-metrics-and-effects.md` | T00, T02 | T01, T05, T06, T07 | open |
| T11 | Integration: the first green `just check` from a fresh clone and in CI, branch protection read back | `T11-integration.md` | T01, T02, T05, T06, T07, T10 | none | open |
| T12 | Maintainer docs: README, CHANGELOG, SECURITY, the crate READMEs | `T12-maintainer-docs.md` | T11 | T13, T15 to T17 | open |
| T13 | Container image: a multi-stage Dockerfile, `just image`, a CI build | `T13-container-image.md` | T11 | T12, T14 to T16 | open |
| T14 | Release process: cargo-release, per-crate tags, a GitHub Release, semver checks | `T14-release.md` | T11, T12 | T13, T15, T16 | open |
| T15 | Mutation testing: cargo-mutants as a weekly report | `T15-mutation-testing.md` | T11 | T12 to T14, T16 | open |
| T16 | Benchmarks and fuzzing: a divan bench, a fuzz crate on stable, a weekly fuzz job | `T16-bench-and-fuzz.md` | T11 | T12 to T15 | open |
| T17 | Renovate: `renovate.json` over every pin, the how-to and the record | `T17-renovate.md` | T13, T14, T15, T16 | T12 | open |
| T18 | Verification gaps: the checks wave 0 and wave 1 never ran, run or recorded as open | `T18-verification-gaps.md` | T11 | T12 to T17 | open |

The ids T03, T04, T08 and T09 are intentionally unused: T03 was merged into T00, T04
into T01, and T08 and T09 into T07.

## Dependency graph

```text
T00 ─┬─ T01 ────────────┐
     ├─ T02 ── T10 ─────┤
     └─ T05, T06, T07 ──┴─ T11 ─┬─ T12 ── T14 ─┐
                                ├─ T13 ────────┤
                                ├─ T15 ────────┼─ T17
                                ├─ T16 ────────┘
                                └─ T18
```

The graph is acyclic: T00, then six wave-1 lanes, then T11, then wave 2.

- **Wave 0, T00.** The tracer bullet through every layer. Nothing else can start until it
  has merged, because every lane replaces stubs it creates and every lane's definition of
  done is `just check` green, which only T00 can make true first.
- **Wave 1, T01, T02, T05, T06, T07 and T10.** Parallel lanes that share no path
  (CONVENTIONS.md §3, §11), with the frozen files frozen. T10 waits for T02 because the
  clippy configuration it builds the bans on is T02's; a libpachinko `clippy.toml`, if
  T10 chooses one, restates T02's root keys.
- **T11** reconciles the follow-ups wave 1 handed back to `main` and proves the whole gate
  from a fresh clone and in CI.
- **T12** writes the entry points last, so they describe what exists. T14 waits for it,
  because it reshapes `CHANGELOG.md` and `SECURITY.md`, which T12 writes.
- **Wave 2, T13 to T16.** The four extras (CONVENTIONS.md §1, decision 8). The frozen list
  lifts after T11; each wave-2 ticket edits only its own lines in the shared files and
  merges one at a time, rebased on the last (CONVENTIONS.md §11). Each adapts a design L's
  spikes already drafted (CONVENTIONS.md §0).
- **T17** comes last because Renovate has to know every pin, and T13 to T16 add pins: the
  Docker digests, the release and mutation tools, the fuzz crate.
- **T18** runs, or records as open, the checks the wave-0 and wave-1 hand-backs did not
  run and T11 could not close. It is off the critical path and blocks nothing.

## How to pick up a ticket

1. Create a worktree on the branch the ticket's `branch:` field names, from `main`,
   after every ticket it depends on has merged. `<branch>` is that field
   (`ticket/t05-agent-contract`) and `<id>` the ticket id (`T05`):

   ```sh
   supacode repo worktree-new --branch <branch> --base main --name <id>
   ```

   Outside a Supacode terminal:

   ```sh
   git worktree add ../<id> -b <branch> main
   ```

   Then run `just initialize` in the new worktree (network, authorised), except in T00,
   which creates the `Justfile`. The `Justfile` puts the worktree's own gitignored
   `.pixi/` and `.tools/` on `PATH`, so a new worktree has no tools until it runs.
   L `scripts/initialize.sh` skips `install-hooks` in a secondary worktree, so once,
   after T00 merges and before the first wave-1 worktree, run `just initialize` in the
   primary checkout on an up-to-date `main`. Only that run installs the shared hook shim
   and prepares the hook environments.

   Except in T00, run `just check` before changing anything, and quote its last line in
   the hand-back notes as the baseline. If it is red, stop: `main` is broken, and the fix
   is a `main` pull request, not part of this lane (`CONVENTIONS.md` §11).

2. Read `CONVENTIONS.md`, then the ticket. Read the L, B and T files the ticket names at
   the commits `CONVENTIONS.md` §0 pins, with `git -C <clone> show <commit>:<path>`.
   Never modify a clone.
3. Edit only the files the ticket lists, plus the `status:` line and hand-back notes of
   the ticket itself. A change needed elsewhere is handed back, not made.
4. Run the ticket's verification commands and quote their output in the hand-back notes,
   each headed by the commit it ran at and showing its exit status. A check that passes by
   printing nothing is preceded by its positive control, whether or not the ticket writes
   one out (`CONVENTIONS.md` §11).
5. At each **Adversarial review** the ticket marks, follow `CONVENTIONS.md` §14 before
   going on.
6. Commit on the ticket branch. Before asking to push, `git fetch origin` (network,
   authorised), rebase onto `origin/main`, rerun `just check`, and rerun the ticket's
   verification commands, replacing each quote with one headed by the rebased commit
   (`CONVENTIONS.md` §11). If it goes red only after the rebase, hand the conflict back
   instead of editing another lane's files.
7. Pushing and opening the pull request are separately authorised: stop and ask.

## Definition of done

`just check` is green in this repository, on the ticket branch rebased onto the current
`main`: its output ends with `All checks passed and the worktree is unchanged.`, and
`git status --short` prints nothing after the ticket's commit. The ticket's acceptance
criteria are met, its verification commands ran with the output quoted in the hand-back
notes, the adversarial reviews the ticket names have run (`CONVENTIONS.md` §14) and every
valid finding is fixed or handed back, every open point is answered or explicitly
carried forward, and the ticket's `status:` is `done` in the same pull request. A
ticket is done when its gate passes, not when its files exist.

## Ticket format

Every ticket carries frontmatter (`id`, `title`, `status`, `depends_on`, `parallel_with`,
`branch`, `estimated_size`) and these sections in this order: Context, Goal, Non-goals,
Files touched, Steps, Acceptance criteria, Verification, Hand-back notes, Open points.
Files touched is a table whose first cell is the path as a code span. Steps mark each
review checkpoint with a bold **Adversarial review** paragraph that cites
`CONVENTIONS.md` §14. Repository paths and URLs are written as code spans, never as
links, because the hook gate runs lychee offline over this directory. A source file is
cited as its letter and a code span, L `Justfile`, so that every citation can be checked
against its pinned commit.
