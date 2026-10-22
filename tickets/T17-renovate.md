---
id: T17
title: "Renovate: `renovate.json` over every pin, the how-to and the record"
status: open
depends_on: [T13, T14, T15, T16]
parallel_with: [T12]
branch: ticket/t17-renovate
estimated_size: M
---

# T17: Renovate: `renovate.json` over every pin, the how-to and the record

## Context

L's spike S01 compared Dependabot, Renovate and nothing, recommended Renovate through the
Mend app on condition the maintainer accepts its write access, and drafted the build
ticket: L `tickets/S01-dependency-updates.md` lines 666-905, with the evidence above it
from line 164 and the `renovate.json` draft from line 474. This ticket adapts that draft.
The maintainer has chosen Renovate for this repository (`CONVENTIONS.md` §1 decision 8).

It comes last because Renovate has to know every pin, and wave 2 added pins: the Docker
base digests (T13), the release and semver tools (T14), cargo-mutants (T15), divan,
cargo-fuzz and the fuzz crate's own lockfile (T16).

## Goal

`renovate.json` on `main` covers every pin in the repository, through Renovate's native
managers where one exists and a regex manager proved by a real pull request where one
does not; the Mend app is installed on `BiscuitBin/pachinko` only, with its permissions on
record in the security model; the how-to and the maintenance page say what the updater
proposes and what stays manual; and decision 0018 records it.

## Non-goals

- No automerge (the draft's `"automerge": false`).
- No Dependabot. It is the draft's fallback; if the app is declined, stop and hand back.
- No pin moved by this ticket's own hand. Renovate's first pull requests are the proof.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `renovate.json` | new | the draft's, extended for wave 2's pins |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | the draft's changes |
| `docs/operations/maintenance.md` | shared, own lines | the draft's changes |
| `docs/explanation/security-model.md` | shared, own lines | the app as a named holder of permissions |
| `docs/decisions/0018-renovate.md` | new | `CONVENTIONS.md` §8 |
| `docs/manifest.yml` | shared, own lines | one entry |
| `docs/README.md` | shared, own lines | one link |
| `docs/decisions/README.md` | shared, own lines | one row |
| `CHANGELOG.md` | shared, own lines | one `[Unreleased]` line |
| `tickets/CONVENTIONS.md` | correction | §12 claims 7 to 9 outcomes, through a `main` pull request |
| `tickets/T17-renovate.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T13 to T16 have merged.
2. **Inventory every pin** on `main`, by file: `Cargo.toml` and both crate manifests,
   `Cargo.lock`, `fuzz/Cargo.toml` and `fuzz/Cargo.lock`, `rust-toolchain.toml` and
   `rust-version`, `pyproject.toml` (pixi pins and the B tag), `pixi.lock`, `tools.txt`,
   `tools-source.txt`, both prek configurations' revs, every remote `uses:` in
   `.github/workflows/` and `.github/actions/` (the local setup action has no pin), `setup-pixi`'s `pixi-version`, the
   `Dockerfile`'s `FROM` digests, and the allium pin inside B. Mark each: native manager,
   regex manager, or manual with a reason.
3. **`renovate.json`.** The draft's, with: the `Dockerfile` (Renovate's native
   `dockerfile` manager handles `tag@digest`); `fuzz/` as a second cargo root;
   `tools-source.txt` beside `tools.txt` in the regex manager; and every new tool pin from
   wave 2. Validate with `npx --yes --package renovate -- renovate-config-validator`
   (network, authorised) or the equivalent the draft names.

   The toolchain moves in one pull request, in three places: `rust-toolchain.toml`'s
   `channel`, `[workspace.package] rust-version`, and the `Dockerfile`'s build-stage
   `FROM rust:<x.y.z>-slim-bookworm@sha256:…`. T13's toolchain check fails the required
   `image` job whenever the channel moves to a release whose image is not on Docker Hub
   yet, so grouping alone is not enough: take all three versions from the `rust` image's
   tags (a regex manager reading `rust-toolchain.toml` with `datasourceTemplate: docker`
   and `depNameTemplate: rust`, with the native `rust-toolchain` reading switched off),
   or hold the group with a `minimumReleaseAge` long enough for the image to publish.
   Say which in the rule's `description`.

   setup-pixi's `pixi-version` input: the draft keeps Renovate's native reading and
   falls back to a regex manager only if the input is unread (L
   `tickets/S01-dependency-updates.md` lines 592-598). Native reading has been seen to
   list the input under `prefix-dev/pixi` and propose nothing while a newer pixi was out.
   Read the input with a regex manager from the start, under the same depName and
   datasource as the `requires-pixi` floor, in the one pixi group, and switch the native
   reading off with a `packageRules` entry.

   **Adversarial review** (`CONVENTIONS.md` §14): the pin inventory, `renovate.json` and
   the validator output, before step 4 asks. The review goes into the authorisation
   request. Ask: which pin on `main` is missing from the inventory; whether each
   manager's pattern matches the pins it claims, and only those; whether the validator
   output is clean; whether a single Renovate pull request can move the toolchain
   channel without the `Dockerfile`'s `rust` tag, or the reverse.
4. **Authorisation required: install the Mend app on `BiscuitBin/pachinko` only.** This
   is an organisation-level action; an organisation owner may have to perform it. Before
   asking, write the permissions the installation screen lists into
   `docs/explanation/security-model.md`, so the trust boundary is on record before the
   app has access (the draft's step 1). That entry does not repeat the draft's claim (L
   `tickets/S01-dependency-updates.md` line 748) that with every update held the app
   produces only the dashboard issue: Renovate's vulnerability-fix pull requests skip
   dashboard approval by default, so once Renovate receives vulnerability alerts, a
   security-fix pull request can open unasked. It still never merges itself.
5. **Land `renovate.json`** through a pull request (separately authorised), then read the
   onboarding pull request or dependency dashboard Renovate opens. Each regex manager is
   proved by the dashboard listing its dependency, or by a real update pull request; quote
   the evidence per manager, as the draft requires.
6. **The docs.** The draft's changes to the how-to and the maintenance page, with this
   repository's pins; decision 0018: Renovate over Dependabot (the draft's evidence), the
   app's access, what stays manual, what would reopen it. The how-to, decision 0018 and
   every `description` in `renovate.json` that says updates wait for approval name the
   vulnerability-fix exception.
7. **The last claims.** Carry the outcomes of `CONVENTIONS.md` §12 claims 7, 8 and 9 from
   the hand-back notes of T14, T16 and T13 into §12's table through a `main` pull request
   (separately authorised), in the shape T11 used for the others. This does not depend on
   the app: if step 4 stops, do this step before handing back.
8. `just check`, the final adversarial review, `just check` again if fixes landed, then
   commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff. Ask: whether the
   how-to and decision 0018 match what was installed and proved; whether the outcomes
   carried for `CONVENTIONS.md` §12 claims 7 to 9 match the hand-back notes of T14, T16
   and T13; whether any text claims every update waits for approval without naming the
   vulnerability-fix exception.

## Acceptance criteria

- Every pin in step 2's inventory is either covered (with quoted evidence from the
  dashboard or a pull request) or listed as manual with a reason in the how-to.
- The app is installed on this repository only, and its permissions are in the security
  model.
- `renovate.json` validates.
- `CONVENTIONS.md` §12 claims 7, 8 and 9 have outcomes, each citing the ticket whose
  hand-back notes hold the evidence.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
npx --yes --package renovate -- renovate-config-validator renovate.json
gh api repos/BiscuitBin/pachinko/installation --jq '.app_slug'
just check-docs
just check
```

## Hand-back notes

To be written by the executing agent: the inventory, the validator output, the
authorisation and who performed the installation, the per-manager evidence, the review
record for each checkpoint (`CONVENTIONS.md` §14), and where this ticket departed from
L's draft.

## Open points

None. Step 3 groups the toolchain with `rust-version` and the `Dockerfile`'s `rust`
image.
