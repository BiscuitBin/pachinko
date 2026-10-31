# Tickets for bootstrapping `pachinko`

This directory is the work breakdown that bootstrapped `BiscuitBin/pachinko`, a Rust
workspace of a library, `libpachinko`, and a service built around it, `pachinko`. These
tickets carried libpawdoku's toolchain, gates, handbook, agent contract, CI and Allium
tooling across, so that the specifications and the code would land in a repository that
was already green.

The bootstrap is done: T00 to T18 are `done` and merged on `main`, the last at
`f080d62`, where `just check` was green. T19 opens the domain: the first Allium module
under `docs/specs/`, with the implementation to follow it. The bootstrap tickets stay
here as the record of what was carried and what was changed.

Each ticket was written for an AI agent with no other context, working in its own git
worktree. `CONVENTIONS.md` is the shared design every ticket obeys and cites by section;
read it first, then the ticket.

Pushing, opening pull requests, tagging, filing these as GitHub issues, changing
repository or organisation settings, installing a GitHub App and editing any other
repository are separately authorised actions. None of these tickets was filed as a
GitHub issue; each landed through a pull request on `main`.

## Index

The `status:` field in each ticket's frontmatter is authoritative; this table is a
snapshot.

| Id | Title | File | Depends on | Parallel with | Status |
| --- | --- | --- | --- | --- | --- |
| T00 | Foundation: a shape-complete skeleton that passes `just check` on its first commit | `T00-foundation.md` | none | none | done |
| T01 | GitHub repository and CI: settings, branch protection, Pages, workflows | `T01-github-and-ci.md` | T00 | T02, T05, T06, T07, T10 | done |
| T02 | Rust quality gate: final tool configs, the std lint table, every Rust recipe green on real code | `T02-rust-gate.md` | T00 | T01, T05, T06, T07 | done |
| T05 | Agent contract: AGENTS.md, fourteen skills, the Allium vendoring | `T05-agent-contract.md` | T00 | T01, T02, T06, T07, T10 | done |
| T06 | Allium gate: the placeholder module, the spec recipes, the two spec pages | `T06-allium-gate.md` | T00 | T01, T02, T05, T07, T10 | done |
| T07 | Handbook: every page, and decision records 0001 to 0014 | `T07-handbook.md` | T00 | T01, T02, T05, T06, T10 | done |
| T10 | Metrics and the effects boundary: rustqual over both crates, the libpachinko bans, the probe | `T10-metrics-and-effects.md` | T00, T02 | T01, T05, T06, T07 | done |
| T11 | Integration: the first green `just check` from a fresh clone and in CI, branch protection read back | `T11-integration.md` | T01, T02, T05, T06, T07, T10 | none | done |
| T12 | Maintainer docs: README, CHANGELOG, SECURITY, the crate READMEs | `T12-maintainer-docs.md` | T11 | T13, T15 to T17 | done |
| T13 | Container image: a multi-stage Dockerfile, `just image`, a CI build | `T13-container-image.md` | T11 | T12, T14 to T16 | done |
| T14 | Release process: cargo-release, per-crate tags, a GitHub Release, semver checks | `T14-release.md` | T11, T12 | T13, T15, T16 | done |
| T15 | Mutation testing: cargo-mutants as a weekly report | `T15-mutation-testing.md` | T11 | T12 to T14, T16 | done |
| T16 | Benchmarks and fuzzing: a divan bench, a fuzz crate on stable, a weekly fuzz job | `T16-bench-and-fuzz.md` | T11 | T12 to T15 | done |
| T17 | Renovate: `renovate.json` over every pin, the how-to and the record | `T17-renovate.md` | T13, T14, T15, T16 | T12 | done |
| T18 | Verification gaps: the checks wave 0 and wave 1 never ran, run or recorded as open | `T18-verification-gaps.md` | T11 | T12 to T17 | done |
| T19 | Pachinko domain: the first behavioural module | `T19-pachinko-domain.md` | T18 | none | todo |

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

The graph was acyclic: T00, then six wave-1 lanes, then T11, then wave 2.

- **Wave 0, T00.** The tracer bullet through every layer. Nothing else could start until
  it had merged, because every lane replaced stubs it created and every lane's definition
  of done was `just check` green, which only T00 could make true first.
- **Wave 1, T01, T02, T05, T06, T07 and T10.** Parallel lanes that shared no path
  (CONVENTIONS.md §3, §11), with the frozen files held frozen. T10 waited for T02
  because the clippy configuration it built the bans on is T02's; T10's
  `crates/libpachinko/clippy.toml` restates T02's root keys.
- **T11** reconciled the follow-ups wave 1 handed back to `main` and proved the whole gate
  from a fresh clone and in CI.
- **T12** wrote the entry points last, so they describe what exists. T14 waited for it,
  because it reshaped `CHANGELOG.md` and `SECURITY.md`, which T12 wrote.
- **Wave 2, T13 to T16.** The four extras (CONVENTIONS.md §1, decision 8). The frozen list
  lifted after T11; each wave-2 ticket edited only its own lines in the shared files and
  merged one at a time, brought up to date with the last by a rebase or a merge of `main`
  (CONVENTIONS.md §11). Each adapted a design L's spikes had already drafted
  (CONVENTIONS.md §0).
- **T17** came last because Renovate has to know every pin, and T13 to T16 added pins: the
  Docker digests, the release and mutation tools, the fuzz crate.
- **T18** ran, or recorded as open, the checks the wave-0 and wave-1 hand-backs did not
  run and T11 could not close. It was off the critical path and blocked nothing.
- **T19** is the first ticket after the bootstrap. It depends on T18 only so that it
  starts from the `main` the bootstrap finished on, and it writes the domain's first
  specification; the implementation tickets that follow depend on it.

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
   L `scripts/initialize.sh` skips `install-hooks` in a secondary worktree, so
   `just initialize` runs once in the primary checkout, on an up-to-date `main`, before
   its first worktree. Only that run installs the shared hook shim and prepares the hook
   environments. The bootstrap's primary checkout ran it for T18 item 1, at `7dbed57`.

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
   authorised), rebase onto, or merge, `origin/main`, rerun `just check`, and rerun the
   ticket's verification commands, replacing each quote with one headed by the resulting
   commit (`CONVENTIONS.md` §11). If it goes red only after the rebase or merge, hand the
   conflict back instead of editing another lane's files.
7. Pushing and opening the pull request are separately authorised: stop and ask.

## Definition of done

`just check` is green in this repository, on the ticket branch rebased onto, or merged
with, the current `main`: its output ends with
`All checks passed and the worktree is unchanged.`, and `git status --short` prints
nothing after the ticket's commit. The ticket's acceptance
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
