---
title: "Deploy to GitHub Pages"
kind: "how-to"
audience: [maintainer, operator, agent]
canonical_for: [deployment_procedure]
requires: []
---

# Deploy to GitHub Pages

This repository publishes one thing: the API reference rustdoc generates for both crates,
`libpachinko` and `pachinko`, built from `main`. `.github/workflows/pages.yml` builds it
with `just site` on every push to `main` and hands `target/doc` to the Pages deployment
action; nothing is committed to a branch.

The site is a project site at <https://biscuitbin.github.io/pachinko/>: the repository
`BiscuitBin/pachinko`, beneath `/pachinko/` on its owner's Pages host. Confirm the
address with `gh api repos/BiscuitBin/pachinko/pages --jq .html_url`, which prints the
one Pages serves.

The site documents `main`, so it may describe an item no release has. Neither crate is
published to a registry (`publish = false` in the workspace manifest), so this site is
the only hosted reference; a release tags a crate and cuts a GitHub Release, and
publishes nothing to a registry ([decision 0016](../decisions/0016-release-process.md)).

## One-time setup

Two repository settings, each made once, before the first push that should deploy. The
workflow can make neither for itself.

1. The Pages source is **GitHub Actions**:

   ```console
   gh api -X POST repos/BiscuitBin/pachinko/pages -f build_type=workflow
   gh api repos/BiscuitBin/pachinko/pages --jq '.build_type, .html_url'
   ```

   The second line prints `workflow` and the address. Where a site exists already the
   `POST` answers HTTP 409, and the same body goes by `PUT`.

2. The `github-pages` environment, which the deploy job names, lets only `main` deploy:

   ```console
   gh api repos/BiscuitBin/pachinko/environments/github-pages --jq .deployment_branch_policy
   gh api repos/BiscuitBin/pachinko/environments/github-pages/deployment-branch-policies --jq '.branch_policies[].name'
   ```

   When this repository's site was created, on 2026-10-23, GitHub made the environment
   with that rule already on it: a custom branch policy that admits `main` alone. A rule
   of `null` admits every branch. If the first line ever prints that, or the environment
   is missing, set a rule that admits protected branches alone, and `main` is the one
   protected branch:

   ```console
   printf '%s\n' '{"deployment_branch_policy": {"protected_branches": true, "custom_branch_policies": false}}' | gh api -X PUT repos/BiscuitBin/pachinko/environments/github-pages --input -
   ```

Until the source is set, the build job succeeds and uploads its artefact but the deploy
job fails with `Failed to create deployment (status: 404)`, even though the workflow
itself is correct.

## Where the site is served from

A project site lives beneath the repository's name on the owner's Pages host. Rustdoc's
links are relative, and each page carries its own path back to the root, so the build is
told nothing about `/pachinko/` and the same tree opens from a local directory.

This repository sets no custom domain, and adding one would be a change to the
repository's settings alone, for the same reason. The `github.io` address is the one
this handbook gives, because it is the one the Pages settings report.

## What the workflow does

- **`build`** checks the repository out, runs the composite setup action, then
  `just check-toolchain` and `just site`. `just site` empties `target/doc`, runs
  `just doc`, the gate's own recipe with the gate's own flags, and writes a root
  `index.html` that links to `libpachinko/index.html` and `pachinko/index.html`, because
  stable rustdoc writes no page at the root. The job uploads `target/doc` as the Pages
  artefact; the upload leaves dotfiles out, so cargo's `.lock` file is not published.
  This job holds `contents: read` and nothing else.
- **`deploy`** publishes the artefact. It holds `pages: write` and `id-token: write`, the
  only write scopes outside `release.yml`, whose `publish` job holds `contents: write`
  ([decision 0016](../decisions/0016-release-process.md)), and it checks nothing out
  and runs no code from the repository. It proves itself to Pages with the run's own
  identity token; no secret is stored. It runs only when the ref is `main`.

A push to `main` deploys, and so does a run started by hand from `main`. A run started by
hand from any other branch builds, and deploys nothing: the deploy job's own `if:` skips
it, and the environment's branch rule would refuse it besides.

One kind of branch slips past the first of those. GitHub compares strings in a workflow
expression, and the names of concurrency groups, without regard to case, and Git allows
a branch named `Main` beside `main`. A run started by hand from such a branch passes the
`if:` and shares `main`'s group, so it can take the place of a waiting deployment of
`main`, and the environment's rule is the one guard left against its publishing. Making
such a branch takes write access to the repository. Do not make one.

Deployments are serialised by a concurrency group and never cancelled mid-flight: a
half-published site is worse than a slightly stale one. The group carries the ref. A
group keeps one run waiting and a newer run takes its place, so with one group for every
ref a hand-started run from another branch, which deploys nothing, could take the place
of a waiting deployment of `main`.

Neither job is a gate. The workflow sits outside the `check` job that protects `main`,
and a failed deployment blocks no merge; see [Quality gates](../reference/quality-gates.md).

## Reproduce a deployment locally

```console
just site
```

Then open `target/doc/index.html` in a browser. It shows the same two links, one to each
crate's page, as the site's root does.

## Rolling back

Re-run the last good deployment from the Actions tab, or revert the commit and let the
push trigger a fresh build. There is no state to migrate and no cache to clear beyond the
browser's.

## Related pages

- [Decision 0013: The API reference on GitHub Pages](../decisions/0013-api-reference-on-github-pages.md)
- [API reference](../reference/api.md)
- [Quality gates](../reference/quality-gates.md)
- [Maintenance](../operations/maintenance.md)
