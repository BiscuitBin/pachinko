---
id: T12
title: "Maintainer docs: README, CHANGELOG, SECURITY, the crate READMEs"
status: open
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

To be written by the executing agent: the link check output, the review record for each
checkpoint (`CONVENTIONS.md` §14), and anything handed back.

## Open points

None.
