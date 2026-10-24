---
id: T17
title: "Renovate: `renovate.json` over every pin, the how-to and the record"
status: in-progress
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
  model. (Departed from at the maintainer's choice: the app is installed organisation-wide;
  see the hand-back notes.)
- `renovate.json` validates.
- `CONVENTIONS.md` §12 claims 7, 8 and 9 have outcomes, each citing the ticket whose
  hand-back notes hold the evidence.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
npx --yes --package renovate -- renovate-config-validator renovate.json
gh api orgs/BiscuitBin/installations --jq '.installations[].app_slug'
just check-docs
just check
```

## Hand-back notes

Run on 2026-10-23 in the Supacode worktree for this ticket. Steps 2, 3, 6 and 7 are
done and committed on the lane branch; steps 4 and 5 wait for authorisation (below), so
`status:` is `in-progress`, not `done`.

### Baseline — `5748060`, untouched

```text
$ just check
...
All checks passed and the worktree is unchanged.
```

### Step 2: the inventory — `5748060`

Read from every tracked file (`git ls-files`, then a search for versions, SHAs and
digests outside `docs/`, `tickets/` and the vendored reference). Beyond the ticket's
list it found `nextest-version` in `.config/nextest.toml`, `rust-version` in
`fuzz/Cargo.toml`, `requires-python` (the review's T17-1-b1) and the fixture manifests
under `tests/`.

| Pin | File | Coverage |
| --- | --- | --- |
| `divan = "0.1.21"` (workspace dev-dependency) and its closure | `Cargo.toml`, `crates/libpachinko/Cargo.toml`, `Cargo.lock` | native `cargo`, group `crates` |
| `libpachinko = { path, version = "0.1.0" }` | `crates/pachinko/Cargo.toml` | not a pin: a path dependency, which the cargo manager skips |
| `libfuzzer-sys = "0.4.13"` and its closure; `libpachinko` path | `fuzz/Cargo.toml`, `fuzz/Cargo.lock` | native `cargo` (a second root), group `crates` |
| `channel = "1.99.1"` | `rust-toolchain.toml` | regex, docker datasource, depName `rust`, only `x.y.z-slim-bookworm` tags (`extractVersionTemplate`); native `rust-toolchain` off; group `rust toolchain` |
| `rust-version = "1.99"` | `Cargo.toml`, `fuzz/Cargo.toml` | regex, docker datasource, depName `rust`, only `x.y-slim-bookworm` tags (`extractVersionTemplate`); group `rust toolchain` |
| `FROM rust:1.99.1-slim-bookworm@sha256:86234e85…` | `Dockerfile` | native `dockerfile`, depName `rust`; group `rust toolchain` |
| `FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79…` | `Dockerfile` | native `dockerfile`, digest only; group `distroless base` |
| 11 exact conda pins (`just`, `prek`, `cargo-binstall`, `cargo-nextest`, `cargo-llvm-cov`, `cargo-deny`, `cargo-insta`, `cargo-mutants`, `cargo-semver-checks`, `cargo-shear`, `taplo`) and `python = "3.14.*"` | `pyproject.toml` | native `pixi`, held; the relock of `pixi.lock` by the hosted app is unproven |
| `requires-python = ">=3.14"` | `pyproject.toml` | native `pep621` (its only dependency here; `[project] dependencies` is empty and it does not read `[tool.pixi]`), disabled by rule: pixi owns Python, so the floor moves by hand with `python = "3.14.*"` |
| `pixi.lock` | `pixi.lock` | `lockFileMaintenance`, held |
| `nextest-version = "0.9.146"` | `.config/nextest.toml` | regex, `conda` datasource `conda-forge/cargo-nextest`, depName `cargo-nextest`; grouped with the pixi pin |
| `biscuit-games-tooling` tag `v0.3.0` | `pyproject.toml` | regex, `github-tags` |
| `requires-pixi = ">=0.81.0"` | `pyproject.toml` | regex, `github-releases` `prefix-dev/pixi`; group `pixi bootstrap` |
| `pixi-version: v0.81.0` | `.github/actions/setup/action.yml` | regex, same depName and datasource; group `pixi bootstrap`; the github-actions manager's reading of `prefix-dev/pixi` is disabled |
| `cargo-hack@0.6.45`, `cargo-release@1.1.7`, `cargo-fuzz@0.13.2` | `tools.txt` | regex, `crate`, held |
| `rustqual@1.8.3` | `tools-source.txt` | regex, `crate`, held |
| 6 hook revs (editorconfig-checker, markdownlint-cli2, typos, shellcheck-py, actionlint, ripsecrets) | `.pre-commit-config.yaml` | regex (SHA and `# vX` comment); native `pre-commit` off; group `hook revs` |
| markdownlint-cli2 rev, repeated | `.pre-commit-fix.yaml` | the same regex and group |
| lychee rev `lychee-v0.24.2` and `LYCHEE_VERSION=0.24.2` twice | `.pre-commit-config.yaml` | manual: the argument must match the rev, and the tag prefix is `lychee-v`; the regex skips its shape and a rule disables the depName |
| `actions/checkout`, `actions/upload-artifact`, `actions/upload-pages-artifact`, `actions/deploy-pages`, `docker/setup-buildx-action`, `docker/build-push-action` | `.github/workflows/*.yml` | native `github-actions`, group `github actions` |
| `prefix-dev/setup-pixi`, `Swatinem/rust-cache`, `actions/cache` (twice) | `.github/actions/setup/action.yml` | native `github-actions`, group `github actions` |
| `uses: ./.github/actions/setup` | workflows | not a pin: a local path |
| allium version and per-artefact SHA-256 | inside `biscuit-games-tooling` | manual: moves with the package tag |
| `tests/fixtures/effects-violation/**` manifests and lockfile | `tests/fixtures/` | not pins: path-only stubs; skipped by `config:recommended`'s `:ignoreModulesAndTests` (`**/tests/**`) |
| `runs-on: ubuntu-latest`; version numbers in comments and prose | workflows, docs | not pins |

### Step 3: `renovate.json` — validator and pattern control

`renovate-config-validator` from `renovate@44.181.1`, the current release on the day
(`npm view renovate version`). A bare `--package renovate` resolved a cached 37.440.7,
which predates `managerFilePatterns` and the `pixi` and `rust-toolchain` managers and
rejected the file, so the version is pinned in the command. With `renovate.json` as an
argument the validator checks it as *global* configuration; with no argument, from the
repository root, it finds and checks it as repository configuration, which is the
relevant run. After review 1's fixes:

```text
$ npx --yes --package renovate@44.181.1 -- renovate-config-validator           (exit 0)
 INFO: Validating renovate.json
 INFO: Config validated successfully against 1 file(s)
$ npx --yes --package renovate@44.181.1 -- renovate-config-validator --strict  (exit 0)
 INFO: Validating renovate.json
 INFO: Config validated successfully against 1 file(s)
```

Negative control: a scratch copy with a `fileMatch` key and a `matchManagerz` key under
`ai_tmp/t17/neg/` fails `--strict` with exit 1, a migration warning and
`Invalid configuration option: packageRules[0].matchManagerz`.

Pattern control: `ai_tmp/t17/match.py` loads `renovate.json`, runs each custom manager's
`matchStrings` (named groups converted to Python's form) over every tracked file, and
fails unless each manager's files and matches equal a hard-coded list. A JavaScript
`RegExp` pass over the same files gave the same counts. Neither is RE2 and neither
resolves a release; the triage subagent also ran Renovate 44.181.1's own extractors on
the files, with the same result. The proof that counts is the dashboard after
installation (step 5).

```text
$ python3 -I ai_tmp/t17/match.py .   (exit 0)
[ok] rust-toolchain.toml's channel: rust 1.99.1
[ok] rust-version: Cargo.toml rust 1.99, fuzz/Cargo.toml rust 1.99
[ok] tools.txt and tools-source.txt: rustqual 1.8.3, cargo-hack 0.6.45, cargo-release 1.1.7, cargo-fuzz 0.13.2
[ok] nextest-version: cargo-nextest 0.9.146
[ok] The tooling package's release tag: biscuit-games-tooling v0.3.0
[ok] requires-pixi: prefix-dev/pixi 0.81.0
[ok] setup-pixi's pixi-version: prefix-dev/pixi 0.81.0
[ok] Hook revs: six in .pre-commit-config.yaml (lychee not matched), markdownlint-cli2 in .pre-commit-fix.yaml
8 managers, 0 failing
```

(Lines condensed; matches outside a manager's own files, in tickets and decision
prose, are listed by the script and are never read by Renovate.)

The toolchain answer to step 3's choice: all three places take their versions from the
`rust` image's `-slim-bookworm` tags (regex managers with `datasourceTemplate: docker`,
`depNameTemplate: rust` and an `extractVersionTemplate` keeping only `-slim-bookworm`
tags; the native `rust-toolchain` manager off), not a `minimumReleaseAge`. The rule's
`description` says so.

### Review 1 (`CONVENTIONS.md` §14) — `5748060` with `renovate.json` uncommitted

Codex 0.163.0, `codex exec -m gpt-6-astra -s read-only`, conformance and breakage lenses
in parallel; triage by a separate Claude Opus 5.5 subagent, which applied the fixes.
Raw files in `ai_tmp/review/T17-1/`.

- **T17-1-a1 (medium), conformance; T17-1-b2 (medium), breakage.** The channel,
  `rust-version` and the `Dockerfile` read different tag sets (bare `x.y.z`, bare `x.y`,
  `x.y.z-slim-bookworm`), so one group could move part of the set. **Valid**: reproduced
  with Renovate's docker versioning on synthetic tags. Fixed: both regex managers keep
  only `-slim-bookworm` tags; with only bare tags nothing is proposed, and with both slim
  tags the group proposes `1.100.0`, `1.100` and `1.100.0-slim-bookworm`. The official
  images manifest lists `1.99-slim-bookworm` and `1.99.1-slim-bookworm` as tags of one
  image entry. The rule's description and decision 0018 were rewritten to match.
- **T17-1-b1 (low), breakage.** The native `pep621` manager reads
  `requires-python = ">=3.14"`, missing from the inventory. **Valid.** Fixed: a rule
  disables that dependency type (pixi owns Python), the inventory gains the row, and the
  how-to lists it as manual.
- Both lenses: no other pin missing, every pattern matches only its claimed pins, the
  validator output clean.

No high-severity finding, so no second round. The rerun of the validator and of
`match.py` after the fixes is quoted above.

### Review 2 (`CONVENTIONS.md` §14) — `6c0d1df` with the fixes uncommitted

Conformance and breakage lenses in parallel; triage by a separate Claude Opus 5.5
subagent, which applied the fixes. Raw files in `ai_tmp/review/T17-2/`.

- **T17-2-a1 (medium), conformance; T17-2-b1 (medium), breakage.** Decision 0018 and the
  security model said the app is installed and holds its permissions, and the how-to
  described a working dashboard. **Valid**: the app is not installed (step 4). Fixed:
  each of the four pages says once that the installation is pending (ticket T17), and
  the permissions are those S01 recorded, to be re-read on the day. Step 5's commit
  removes those sentences.
- **T17-2-a2 (low), conformance.** "Neither bot relocks `pixi.lock`" stated as known what
  is unproven for the Mend app. **Valid.** Fixed in decision 0018's context.
- **T17-2-b2 (medium), breakage.** The how-to read any red `rust` job on a toolchain
  branch as new lints, though the job also runs formatting, tests, snapshots and
  documentation. **Valid.** Fixed: read which step failed; `just clippy` for a lint.
- **T17-2-b3 (low), breakage.** The crates and actions rules promised one pull request,
  but `separateMajorMinor` (default `true`) takes priority over a group. **Valid**, per
  Renovate's configuration reference. Fixed in both descriptions and in one how-to
  sentence below the group table.

No high-severity finding, so no second round. After the fixes the validator exits 0
("Config validated successfully against 1 file(s)"), `match.py` reports "8 managers,
0 failing", and `just check-docs` and `just lint` exit 0.

### `code-review` skill (medium) — `6c0d1df` with review 2's fixes uncommitted

Ten unverified findings, each checked by the executing agent:

- Invalid: the pixi manager double-reading `biscuit-games-tooling` (Renovate's pixi
  extractor skips it as `unspecified-version`, review 1's triage); the github-actions
  manager reading `pixi-version` under another name (its extractor, run here, returns
  depName `prefix-dev/pixi`, datasource `github-releases`, depType `uses-with`, so the
  disabling rule matches); the `lockFileMaintenance` schedule repeating the global one
  (L's draft writes it on purpose; harmless).
- Fixed: majors in a group open together in one pull request of their own, not one each
  (`separateMultipleMajor` is off), in both rule descriptions and the how-to; the crates
  description now says a range moves when a release falls outside it; decision 0018 and
  the maintenance page now name `requires-python` among the manual moves, as the how-to
  does; the CHANGELOG line no longer says "every pin"; the how-to tells the toolchain
  branch to update the version the crate READMEs and the `Dockerfile`'s first comment
  repeat, which no manager reads.
- Handed back, outside Files touched: `tools-source.txt:7` still says rustqual is
  "Bumped by hand"; `crates/*/README.md` and `Dockerfile:1` repeat the toolchain version;
  a `tools.txt` bump is resolved on crates.io, which does not prove the release binaries
  `just install-tools` needs exist yet, so the approver checks them before the click.

### Steps 6 and 7

The docs as the ticket lists them, in held-mode wording. Every place that says an update
waits for approval names the vulnerability-fix exception and that it applies once
vulnerability alerts are enabled. The security model names the app's permissions as S01
recorded them (L `tickets/S01-dependency-updates.md`, "Also verified"), to be checked
against the installation screen on the day. `CONVENTIONS.md` §12 claims 7, 8 and 9 have
outcomes citing T14's "Semver and claim 7", T16's Claim 8 paragraph and T13's image-job
evidence for run `40065797710`. §12's intro sentence already says T17 carries them and
is unchanged.

Mentions outside Files touched that now read stale and were left alone (`just
check-docs` passes with them): `docs/decisions/0007-dependency-policy.md:93` ("Nothing
updates any pin automatically until ticket T17 brings Renovate"),
`docs/decisions/0015-container-image.md:26` (the toolchain "move[s] together in T17's
Renovate group", now true) and `docs/reference/quality-gates.md:137` (a wave-2 note).
The first wants a follow-up edit.

### Departures from L's draft and from this ticket

- The toolchain is read from the `rust` image's `-slim-bookworm` tags, not by the native
  `rust-toolchain` manager and the `rust-version` datasource.
- `pixi-version` is a regex manager from the start, with the github-actions reading of
  `prefix-dev/pixi` disabled.
- Added: the `Dockerfile` (the toolchain group and a `distroless base` group),
  `fuzz/Cargo.toml`'s `rust-version`, `tools-source.txt` and wave 2's tools in the
  tool rule, `nextest-version` grouped with the pixi `cargo-nextest` pin through the
  `conda` datasource, and a rule disabling `pep621`'s `requires-python`.
- The validator was run ad hoc through `npx`, as this ticket allows; L's draft wanted a
  `Justfile` recipe instead.
- The `CONVENTIONS.md` §12 edit is committed on this lane branch rather than through a
  separate `main` pull request, at the maintainer's choice.
- The branch is `ticket/T17-renovate`, not the frontmatter's `ticket/t17-renovate`
  (Supacode's naming).

### Final verification — `6c0d1df` with review 2's and `code-review`'s fixes, committed next

```text
$ npx --yes --package renovate@44.181.1 -- renovate-config-validator --strict   (exit 0)
 INFO: Config validated successfully against 1 file(s)
$ python3 -I ai_tmp/t17/match.py .   (exit 0)
8 managers, 0 failing
$ just check   (exit 0)
All checks passed and the worktree is unchanged.
```

### Step 4: installation

The maintainer installed the Mend Renovate app on 2026-10-24, for every repository in the
`BiscuitBin` organisation rather than `BiscuitBin/pachinko` alone, and chose to keep it
that way; the security model, decision 0018 and the acceptance criterion say so. The
ticket's `gh api repos/BiscuitBin/pachinko/installation` answers only to the app's own
token (a user token gets `401 A JSON web token could not be decoded`), so the organisation
listing stands in for it:

```text
$ gh api orgs/BiscuitBin/installations --jq '.installations[] | {app_slug, repository_selection, permissions, events, created_at}'   (exit 0)
{"app_slug":"renovate","created_at":"2026-10-24T13:48:08.000-07:00","events":["issues","issue_comment","pull_request","pull_request_review_comment","push","repository"],"permissions":{"administration":"read","checks":"write","contents":"write","issues":"write","members":"read","metadata":"read","packages":"read","pull_requests":"write","statuses":"write","vulnerability_alerts":"read","workflows":"write"},"repository_selection":"all"}
```

Against S01's list it adds `members: read` and `packages: read`; the pages now carry the
installation's own list. Right after installation `pachinko` had no open issue or pull
request from Renovate.

### Pending authorisation

Each is a separate ask; none has been performed.

1. Push this branch and open its pull request; the merge is the maintainer's.
2. Step 5's evidence, once the pull request has merged: read the dependency dashboard and
   quote, per manager, the detected dependency (or an update pull request), with
   read-only `gh` calls. That commit also drops the four "until `renovate.json` reaches
   `main`" sentences. Expected on the
   dashboard: no lookup warning for the path dependency `libpachinko`, and
   `biscuit-games-tooling` under the regex manager (the pixi manager skips it as
   `unspecified-version`).

## Open points

None. Step 3 groups the toolchain with `rust-version` and the `Dockerfile`'s `rust`
image.
