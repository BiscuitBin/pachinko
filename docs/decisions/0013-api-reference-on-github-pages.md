---
title: "Decision 0013: The API reference on GitHub Pages"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_api_reference_on_pages]
requires: []
---

# Decision 0013: The API reference on GitHub Pages

*Carried from libpawdoku's decision 0016 at `ccdd001`.*

## Context

The API reference is the one rustdoc generates for the two crates, `libpachinko` and
`pachinko`. Without a hosted copy, reading it takes a clone and `just doc`. Every crate
is `publish = false` ([decision 0006](0006-apache-2-0.md)), so docs.rs holds nothing.
The repository is public, and of its workflows `ci.yml` and `audit.yml` hold no scope
beyond `contents: read`: the gate and the advisory audit only read.

The maintainer chose on 2026-10-21 to carry the whole gate and CI across, and the
carried gate already had `just site` and `.github/workflows/pages.yml`, so both exist
from this repository's first commit. The first deployment from `main` was at `49890ae`,
on 2026-10-23.

## Decision

**The site is rustdoc alone, from `main`, on every push, as a project site** beneath
`/pachinko/` on the organisation's Pages host;
[Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md) gives the address. There is
no site generator, and the handbook under `docs/` stays where it is, read on GitHub.

**It is built by `just site`**, which empties `target/doc`, runs `just doc` with the
gate's own flags, and adds a root `index.html` that links to both crates' pages, because
stable rustdoc writes none. No second build of the documentation exists: what is hosted
is what gate 12 proved.

**The deploy has a workflow of its own**, `.github/workflows/pages.yml`. Its build job
holds `contents: read`. Its deploy job holds `pages: write` and `id-token: write`,
checks nothing out and runs no code from the repository.

**The push is the authorisation.** `AGENTS.md` lists pushing and publishing among the
actions that need explicit authorisation each time, and says that once a workflow
deploys from `main`, a push there is a deployment. A deployment here follows from a push
to `main`, whether typed or made by GitHub when a pull request merges. Pushing is on that
list already, and merging a pull request is the maintainer's own act, so no action an
agent takes deploys on its own. Starting the workflow by hand from `main` is a deployment
and is authorised as one.

The workflow guards three places, for one reason: it can be started by hand from a branch
that must not publish.

- The deploy job carries `if: github.ref == 'refs/heads/main'`. A setting can be changed
  where no reviewer sees it; a line in the workflow cannot.
- The concurrency group carries the ref. A group keeps one run waiting and a newer one
  takes its place, so one group for every ref would let a run from another branch, which
  deploys nothing, displace a waiting deployment of `main`.
- The `github-pages` environment's branch rule admits `main` alone, and is checked by
  hand, as [Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md) records.

Turned down:

- **A site generator for the handbook.** A second tool and a second build, for a
  handbook whose frontmatter and manifest are made for GitHub's rendering.
- **Publishing on release only.** There would be nothing to read until the first
  release, and the release process is still to come (ticket T14, in wave 2).
- **A tree per version.** docs.rs would do that for releases if a crate were ever
  published, and neither is.

## Consequences

The ones that hurt.

- **The one workflow that holds a write scope.** The
  [security model](../explanation/security-model.md) has a job to describe rather than
  an absence, and no page may say the workflows only read.
- **The site describes `main` and says so nowhere on its pages.** Rustdoc prints the
  crate's version and nothing about the branch, so a reader may find an item no release
  has. Only the handbook says it.
- **The reference would be in two places** if a crate were ever published: `main` here,
  each release on docs.rs.
- **The handbook's explanations are not on the site.** A reader of the hosted reference
  who wants the reasoning follows a path to GitHub.
- **The root index names the crates by hand.** `just site` writes one link per crate, so
  a crate added to the workspace is missing from the root until the recipe changes.
- **A branch that differs from `main` only in case is not told apart.** Expressions and
  concurrency groups ignore case, so a hand-started run from a branch named `Main`
  passes the deploy job's `if:` and shares `main`'s group. The environment's rule is
  then the only guard, and that is the setting the `if:` was written not to lean on.
  Nothing in an expression compares with case, so the workflow cannot close this itself.
- **Two settings live outside the repository.** The Pages source and the environment's
  rule are not in any file, so no gate reads them.
- **The build installs the whole gate's tooling to run one `cargo doc`**, because the
  setup action is the one tested path to `just` and the pinned toolchain.

## What would reopen this

Publishing the handbook as a site. A third crate in the workspace, or a crate whose
reference should not be public. A custom domain for this repository. A decision to
publish a crate, which puts its releases on docs.rs, or a need for versioned
documentation that docs.rs does not meet.

## Related pages

- [Deploy to GitHub Pages](../how-to/deploy-to-github-pages.md)
- [API reference](../reference/api.md)
- [Security model](../explanation/security-model.md)
- [Quality gates](../reference/quality-gates.md)
