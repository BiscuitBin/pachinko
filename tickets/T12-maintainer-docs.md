---
id: T12
title: "Maintainer docs: README, CHANGELOG, SECURITY, the crate READMEs"
status: done
depends_on: [T11]
parallel_with: [T13, T15, T16, T17]
branch: ticket/t12-maintainer-docs
estimated_size: S
---

# T12: Maintainer docs: README, CHANGELOG, SECURITY, the crate READMEs

## Context

The entry points come last so that they describe what exists rather than what was
planned. T00 left five stubs; after T11 the repository is integrated and green. L
`tickets/T10-maintainer-docs.md` is the procedure (L numbered it T10), and L
`README.md`, L `CHANGELOG.md`, L `SECURITY.md` and L `crates/pawdoku/README.md` are the
shapes.

Wave 2 runs beside this ticket and also writes `CHANGELOG.md` lines
(`CONVENTIONS.md` §11, "Wave 2 shares files by rebasing"). This ticket owns the file's
structure; a wave-2 ticket adds its own entry under `[Unreleased]` and rebases. T14
waits for this ticket, then splits `[Unreleased]` per crate and rewrites `SECURITY.md`'s
"Supported versions".

## Goal

A `README.md` that tells a newcomer what the repository is, that the domain is not yet
specified, how to get from clone to green, and where the handbook is; a `CHANGELOG.md` in
Keep a Changelog form with one `[Unreleased]` section describing the foundation; a
`SECURITY.md` that says how to report a vulnerability; and a README for each crate.

## Non-goals

- No handbook page. The README links to `docs/README.md` rather than restating it.
- No badge for a service that does not exist yet (crates.io, docs.rs).
- No release, version bump or tag (T14).

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `README.md` | final | what, status, quick start, handbook, licence |
| `CHANGELOG.md` | final | Keep a Changelog, `[Unreleased]` |
| `SECURITY.md` | final | private vulnerability reporting on `BiscuitBin/pachinko` |
| `crates/libpachinko/README.md` | final | what the library is and is not yet |
| `crates/pachinko/README.md` | final | what the service is and is not yet |
| `tickets/T12-maintainer-docs.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T11 has merged.
2. **`README.md`.** One paragraph on the two crates and the effects boundary; a status
   line saying the domain is defined by the Allium specifications, not yet written; the
   quick start (`just initialize`, `just check`), with the prerequisites linked from
   `docs/how-to/develop-locally.md` as a code-span path; the CI badge for `ci.yml` and a
   link to the Pages API reference; the licence.
3. **`CHANGELOG.md`.** L's header and links, with `BiscuitBin/pachinko` URLs; one
   `[Unreleased]` section listing the foundation in a few lines, not per ticket.
4. **`SECURITY.md`.** L's, with the reporting link pointing at this repository's
   private vulnerability reporting (T01's open point says whether it was enabled; if
   not, name the maintainer's contact the maintainer gives).
5. **The crate READMEs.** Each says what the crate is for, that its API is one item
   today, and where its rustdoc is.
6. `just check-docs`, `just check`, the final adversarial review, `just check` again if
   fixes landed, then commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the five files. Ask: which entry point
   describes a recipe, page, badge, link or feature that does not exist on `main`.

## Acceptance criteria

- Every link in the five files resolves (`just check-links-online`, network, authorised).
- Nothing in them describes a feature that does not exist on `main`.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just check-docs
just check-links-online
just check
```

## Hand-back notes

Completed on `ticket/T12-maintainer-docs`, from `main` at `c1b4bcb` after T11 merged.
Only the five entry points and this ticket changed; no behaviour or dependency changed.

### Baseline and setup

At `c1b4bcb`, untouched tracked worktree:

The initial `just check` could not start because `bg-project-check` was not installed
(exit 127). The authorised `just initialize` then completed (exit 0), including the
pinned Allium checker, and skipped hook installation in this secondary worktree.
Initialization required access to pixi's shared cache outside the sandbox. Commands
used rustup's cargo first on `PATH`, as required by §2. The baseline rerun completed:

```text
$ just check
All checks passed and the worktree is unchanged.
exit 0
$ git status --short
(no output)
exit 0
```

### Sources and adaptations

Read L's procedure and four cited entry points at `ccdd001`, the current source and
manifests, the handbook owners, and T01 and T11's hand-back evidence. T01 records private
vulnerability reporting enabled, so the policy links directly to this repository's
private reporting form. The CI badge names the existing `ci.yml`; the API links name
the deployed Pages root and both crate pages.

The entry points describe two std crates with one public item each and an undefined
domain. L's domain, `no_std` and entropy-ban claims do not apply. Detailed tool
provenance, workflow permission scopes and effects-check limits remain on the linked
canonical security-model page rather than being copied into the root policy. The
changelog has one `[Unreleased]` section, ready for the wave-2 entries and T14's split.

The supplied implement skill requests `code-review`, which is not available as a skill
in this session. Used the repository's `review-change` skill and the independent reviews
below. No test-first seam was needed for these prose changes; the existing gates verify
them.

Branch-name deviation: the existing Supacode branch is `ticket/T12-maintainer-docs`,
while the ticket declares the lowercase `ticket/t12-maintainer-docs`. Preserved the
current branch as the user's implement instruction requires. No branch rename was made.

### Final adversarial review, checkpoint 1

At `c1b4bcb` with uncommitted changes: two fresh-context Claude Code reviewers ran
concurrently with `claude-opus-5-5` in read-only plan mode, after checking `claude --help`.
Both exited 0. They reviewed the diff against the ticket, §11 and §14, the pinned L
files and the actual repository. Neither found an entry point describing a missing
recipe, page, badge, link target or feature.

Separate fresh-context Codex triage agents checked every finding against source and
applied the valid document fixes sequentially:

| Finding | Verdict and resolution |
| --- | --- |
| B01, low; C01, low | Valid duplicate: README's offline claim needed the shared hook environments. Now requires this worktree's tools and hook environments prepared from the primary checkout. |
| B02, medium | Valid evidence requirement: online checking had already passed, but its output still needed recording. Reran it after the fixes and quoted it below. |
| B03, low; C03, low | Valid branch-case deviation: documented above; the user's current-branch instruction takes precedence. |
| C02, low | Valid adaptation gap: added the lockfile-drift check to the security summary; tool provenance and workflow permissions stay on their linked owning page, for the reason above. |

No high-severity finding required a second round. Nothing remains handed back. Raw
findings and combined verdicts are in `ai_tmp/review/T12-1/`, outside the commit.
The executing agent also read the whole diff against the repository invariants and
documentation ownership; no additional finding remained.

### Verification after the review fixes

At `c1b4bcb` with uncommitted changes, after applying both document fixes:

```text
$ just check-docs
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
Validated 39 pages and 40 canonical topics.
exit 0

$ just check-links-online
Lychee online (manual)...................................................Passed
exit 0

$ just check
The worktree matches the check baseline.
All checks passed and the worktree is unchanged.
exit 0
```

The full gate passed all three unit/integration tests and the library doctest, and
reported 100% line coverage. The service README's example was also exercised at
`c1b4bcb` with uncommitted changes: `just build` exited 0, and
`./target/debug/pachinko` printed `pachinko 0.1.0` and exited 0.

## Open points

None.
