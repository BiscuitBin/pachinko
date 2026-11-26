---
id: QS06
title: "Finding out: can the guest image be built and hosted on GitHub"
status: todo
depends_on: [QS05]
parallel_with: []
branch: ticket/qs06-guest-image
estimated_size: S
---

# QS06: Finding out: can the guest image be built and hosted on GitHub

## Context

After QS04, `just studio up` boots the project's machine from a base image pinned by
digest, and a provisioning step installs the guest's packages from a fixed
`snapshot.debian.org` timestamp. That is ruling 10, stated in the README of these
tickets, `.scratch/quint-studio/README.md`. The maintainer's words, on 2026-11-25, were:
"Got with option 1 for now, but create a follow up ticket for later (after mvp) to see
if we can build and host it as an artifact on github." Option 1 was that dated snapshot.
This is the follow-up ticket. It depends on QS05 because the lead reads "after mvp" as
after the first working version, QS02 to QS05; that reading is the lead's.

It is a finding-out ticket, as QS01 was. It changes no tracked file except this ticket
and its Status cell in the README. Everything it builds lives in `ai_tmp/qs06/`, or in
the machine state `just smolvm` keeps outside the worktree (ruling 5), and is thrown
away. Its hand-back notes are the deliverable.

Read these:

- `.scratch/quint-studio/README.md`, "The maintainer's rulings".
- `.scratch/quint-studio/QS04-just-studio.md`, its design and hand-back notes: the base
  image, the snapshot, the provisioning step and the packages it resolved.
- `.scratch/quint-studio/QS02-pinned-smolvm.md`, its hand-back notes, and
  `docs/decisions/0022-quint-studio-machine.md`.
- `tools/quint-studio/Smolfile`, `scripts/studio.sh`, and `just smolvm` in the
  `Justfile`.
- `.scratch/apalache/AP01-java-and-verify.md`, "Adversarial review", and
  `tickets/CONVENTIONS.md` §11, §13 and §14.

## Goal

An answer to the maintainer's question, in its two parts: can the guest image be built,
and can it be hosted as an artifact on GitHub. The guest image is the pinned base with
QS04's packages, which the Smolfile would name by digest in place of the package install
in QS04's provisioning step. The rest of that step, the git wrapper and the extraction
of Studio's AppImage, stays as QS04 wrote it.

The hand-back gives each part its own answer, "built" and "hosted": yes, no, "not
tested" or "not verified", with the reason, and never a yes or a no that the quoted
output of its claims does not show. With a yes, it gives the design a later ticket would
build. It answers the five claims below one by one, each with its evidence or with "not
tested"; claim 5 stays "not verified" unless the maintainer authorised the upload.

## Design proposed in this ticket

These are proposals. Step 1 amends them where QS04's or QS02's hand-back disagrees, and
the maintainer rules at step 4, with checkpoint 1's findings in hand. The workflow that
would build or publish the image is not designed here; that is a later ticket's.

- **The image holds the pinned base and QS04's packages, nothing else.** No file of the
  worktree, no credential, and not Studio's AppImage: ruling 1 pins that separately, and
  its licence for redistribution is unchecked. The packages come only from the snapshot
  QS04 records, never from a live mirror. The image stands in for the package install
  alone: this ticket runs neither the git wrapper's install nor the AppImage's
  extraction, so it does not need Studio installed.
- **The builder is not chosen here.** Step 2 reads the candidates on the day, and the
  plan names one, with what it installs and every place it writes.
- **The candidate** is the first build of step 5 that completes, and the candidate
  digest is the digest of its manifest for the guest's platform. Every later comparison,
  boot and upload names that digest, and with no candidate no upload is offered. It is
  booted locally, through `just smolvm`. If the pinned smolvm cannot take an image that
  no remote registry holds, claim 2 is "not tested" at step 6, with the reason, and an
  authorised upload can still show it at step 7.
- **Writes outside the worktree.** `just smolvm` writes the state directory, with the
  parents it is made under, the state link and the lock, as QS02 landed them. Ruling 5
  authorises those, and each step that writes them names them. `just smolvm` refuses a
  state root of the caller's, so there is no second state: "fresh state" is this
  worktree's state, removed with QS02's removal recipe and made again. Any other write
  outside the worktree, such as a builder's install, its daemon's store or its cache,
  is an "Authorisation required" stop before its first use, whether or not the builder
  is already installed. The plan keeps the builder's cache and output in
  `ai_tmp/qs06/` wherever the builder allows it.
- **Nothing leaves the machine without the maintainer's leave.** Pushing an image,
  uploading a release asset, adding or running a workflow, changing a package's
  visibility, deleting what was uploaded and any other change on GitHub are each an
  "Authorisation required" stop (`tickets/CONVENTIONS.md` §11, "Separately authorised
  actions"). Leave for one is not leave for the next.
- **Risks** (§13, §11). A package's visibility may need an organisation owner
  ("Organisation settings"). `just check` needs no network, and each step that does is
  named. A read of GitHub's API fails closed, since a 404 may be a refusal. No token is
  printed or written to a file ("Credentials").

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 1 | Built: the image builds from the pinned base with QS04's packages; its package sources name only QS04's snapshot, and the version and the origin of each package installed are the ones QS04 recorded | step 5 |
| 2 | Booted: the pinned smolvm boots a new machine from the candidate digest, the guest reports the guest's platform, and QS04's packages are there with no package install run in the machine | step 6; step 7 if only an authorised upload can give smolvm the image; otherwise "not tested" |
| 3 | Reproducible: two builds that share no cache give the same digest for the guest's platform | step 5 |
| 4 | Form: GitHub can hold the image, as a container package or a release asset, in a form the Smolfile can name by digest | step 2, by reading; step 7 if an upload is authorised |
| 5 | Hosted: the candidate, uploaded and then fetched into fresh state by the exact reference the Smolfile would hold, has the candidate digest and boots | step 7, only if an upload is authorised; otherwise "not verified" |

## Non-goals

- No workflow, recipe, script, page or decision, and no change to the Smolfile or to
  QS04's provisioning step, which stays the supported route.
- No upload, and no other change on GitHub, without its own authorisation.
- No AppImage in the image, and no `CHANGELOG.md` entry.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review" (T37 is
still todo). One delta overrides them: the reviewer is Codex Sol, `-m gpt-6.2-sol`, in
place of the `-m gpt-6-astra` that AP01's command and §14 print, by the maintainer's
choice of 2026-11-25. AP01's fallback to a fresh-context `claude-opus-5-5` subagent
still applies, and the hand-back notes record the model that ran. `<id>` is `QS06`.
The checkpoints have fixed numbers:

1. At step 3, over the plan, before anything is built.
2. At step 9, over the hand-back notes with the Verification output in them, before the
   record commit, which is the ticket's final commit.
3. From 3 onwards, in the order reached: one before each separately authorised action
   that is reached, over what its stop will ask for. A local commit is not such an
   action, nor is a write to the state ruling 5 authorises. A push, a pull request, an
   upload, the deletion of an upload, a change to a package's visibility, a workflow
   run and any other write outside the worktree, an install among them, are.

The hand-back records each checkpoint that did not run, and why.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `.scratch/quint-studio/QS06-guest-image.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS06's Status cell |

Everything else lives in `ai_tmp/qs06/`, which Git ignores, or outside the worktree as
"Writes outside the worktree" says, where step 8 removes it.

## Steps

Each step quotes its commands' output and exit status, headed by the commit (§11), with
any token or account identifier redacted as `<redacted>`.

1. **Worktree, baseline and the facts.** Create the worktree on
   `ticket/qs06-guest-image` from a base that holds QS05, which the maintainer names,
   and record the base and its full commit id. Show that the base also holds QS01's
   hand-back, with its "Rulings after the hand-back" section, and the README of these
   tickets with rulings 1 to 10; if either is missing, **stop** for the maintainer, who
   names the base. Run `just initialize` and `just install-smolvm` (network,
   authorised), and `just check` on the untouched worktree, and quote its last line.
   This ticket does not run `just install-quint-studio`: if `just initialize` reports
   that upstream has moved Studio's build, record it and go on. Read the hand-backs
   this ticket cites and write `ai_tmp/qs06/amendments.md`, as QS02's step 1 does.
2. **Read before building** (network, authorised: documentation and public, read-only
   requests; if smolvm's help is read through `just smolvm`, writes outside the
   worktree, authorised by ruling 5: the state, as step 6 names it). Read on the day:
   smolvm's help on how a machine is given its image, from a registry and without one;
   the builders that could make the image; and GitHub's documentation on container
   packages and release assets. Write `ai_tmp/qs06/plan.md`: the image's content; the
   builder, with every place outside `ai_tmp/qs06/` that it or its daemon would write;
   how two builds that share no cache are made and compared; how the pinned smolvm is
   given the candidate with no registry, or that it cannot be; what the Smolfile would
   name in each form GitHub offers; and every action that would leave the machine.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qs06/plan.md` and `ai_tmp/qs06/amendments.md`). Questions:
   - Could the image hold anything but the pinned base and QS04's packages?
   - Could the build reach a live mirror, or install a version QS04 did not record?
   - Could "the same digest" be claimed from one build, a shared cache or two tags?
   - Does any step leave the machine, or write outside ruling 5's state, with no stop?
4. **The ruling.** **Stop** for the maintainer's ruling on the plan, with the
   checkpoint's findings in hand, and record it. **Authorisation required**, for each
   place outside the worktree that the plan's builder would write, before its first
   use: run its checkpoint, ask, and record how it is removed. If the maintainer
   refuses and the plan has no builder left, there is no candidate: claims 1 to 3 are
   "not tested", steps 5 and 6 do not run, and the ticket goes on at step 7.
5. **Build, twice** (network, authorised: the base image and the snapshot; writes
   outside the worktree: ruling 5's state, as step 6 names it, if the build runs through
   `just smolvm`, and otherwise only where step 4 gave leave). Build the image twice,
   with the outputs under `ai_tmp/qs06/`. Each build has a cache and an output of its
   own that start empty, so that the second can read nothing the first wrote: quote
   where each lies and that it was empty. For each build quote its manifest for the
   guest's platform, that manifest's digest, and the platform it names. The first build
   that completes is the candidate. Compare the two digests (claim 3). Digests that
   differ are a finding, not a failure of the ticket: record what differs. For claim 1,
   quote the candidate's package sources, which must name only QS04's snapshot, and the
   version and the origin of each package installed, and compare them with QS04's
   hand-back. If only one build completes, claim 3 is "not tested". If none does, quote
   the failure as claim 1's outcome: there is no candidate, claims 2 and 3 are "not
   tested", step 6 does not run, and the ticket goes on at step 7.
6. **Boot** (network, authorised: the guest's egress; writes outside the worktree,
   authorised by ruling 5: the state directory with the parents it is made under, the
   state link and the lock, nothing else). By hand through `just smolvm`, give the
   pinned smolvm the candidate by the route the plan names, with no registry, and make
   a new machine from the candidate digest. Quote every command run on the machine and
   in it: none installs a package, and none is QS04's provisioning. Show it booted
   (claim 2): the digest the machine was made from is the candidate digest, a command
   in the guest reports the guest's platform, and the guest's package manager lists the
   packages of claim 1 at the versions recorded. Delete that machine, and show it gone.
   If smolvm cannot be given the candidate without a registry, quote that: claim 2 is
   "not tested" here, with the reason, and step 7 may still show it. A machine that is
   made and does not boot is claim 2's outcome, quoted.
7. **Hosting** (network: only what each stop below authorises, and then the guest's
   egress; writes outside the worktree, authorised by ruling 5: the state, as step 6
   names it). Record claim 4 from step 2's reading, as read and not run: a form is said
   to work only where claim 5 is shown for it. With no candidate, the step ends here: no
   upload is offered, and claim 5 is "not verified". With one, each of these is
   **Authorisation required**, separately and after its own checkpoint: an upload of the
   candidate as a container package or a release asset, with the fetch that follows it;
   a change to a package's visibility; a workflow added or run on GitHub, which no
   commit of this ticket carries; and the deletion of what was uploaded. If the
   maintainer declines the upload, claim 5 is "not verified", and claim 2 stays as step
   6 left it. If the upload is authorised:
   - upload the candidate, and quote the reference and the digest GitHub reports;
   - make the state fresh: delete any machine, remove this worktree's state with
     QS02's removal recipe, and show the state directory and the state link gone;
   - through `just smolvm`, give smolvm the image by the exact reference the Smolfile
     would hold in that form, or by the download and import that form needs, show that
     its digest is the candidate digest, and boot it as step 6 does (claim 5, and claim
     2 where step 6 could not show it). Delete that machine, and show it gone;
   - ask whether what was uploaded stays. Delete it only with that leave, and show it
     gone; otherwise record where it is and how to delete it.
8. **Clean up, write the notes and commit.** Remove the images and any download in
   `ai_tmp/qs06/`. Delete any machine left, remove this worktree's state with QS02's
   removal recipe, and show the state directory and the state link gone; name what that
   recipe leaves by QS02's design. Remove everything else the ticket wrote outside the
   worktree, the builder's install, store and cache among them, and show each gone; for
   anything that is to stay, ask the maintainer's leave and record it with how to remove
   it. Write the notes. Set `status: done` here and in
   `.scratch/quint-studio/README.md`, changing nothing else in the README, and commit.
9. **Verification, adversarial review and hand-back.** Run Verification on that commit
   and quote it in the notes, headed by the full commit id. Then run checkpoint 2
   (conformance and breakage, over the notes with their quoted output, Verification's
   included). Questions:
   - Does any outcome, or either answer, say more than its quoted output shows?
   - Is anything said to be hosted, pulled or booted that was only read about?
   - Is anything left outside the worktree, or on GitHub, that the notes do not name?
   - Does any quoted output hold a credential?

   The README's section "Rules every ticket holds" binds this ticket from step 1 on,
   and here it governs the evidence a fix from this checkpoint touches. Then commit the
   record. On the record commit, run `git diff --name-only HEAD~1 HEAD`, which must
   list only this ticket, then `just check` and `git status --short`, and report their
   output to the maintainer with the hand-back, headed by that commit. If any fails,
   fix the cause in a new commit and run all three again. **Stop** before pushing or
   opening a pull request: both need authorisation (§11, "Separately authorised
   actions"), and each, if the maintainer asks for it, has its own checkpoint first.

## Acceptance criteria

- Each of the five claims has its outcome recorded, with quoted output headed by its
  commit, or as "not tested" or "not verified" with the reason.
- The hand-back answers "built" and "hosted" separately, each as yes, no, "not tested"
  or "not verified" with the reason, and neither says more than the quoted output of
  its claims shows. With a yes, it gives the design a later ticket would build.
- Each branch taken is recorded as such: no image built; no route into smolvm without
  a registry; the upload declined. No upload was asked for without a candidate digest.
- Every action that left the machine is named with its authorisation and how to undo
  it, or the hand-back says that none did. No quoted output holds a credential.
- Nothing the ticket built, installed or wrote remains outside the worktree but what
  QS02's removal recipe leaves by design, or what remains is named with the
  maintainer's leave and how to remove it.
- The README's diff against the base changes QS06's Status cell and nothing else.
- Each checkpoint that ran is recorded as §14's Record asks, with AP01's deltas, and
  each that did not run is named with the reason.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record
  commit, as step 9 reports.

## Verification

The ticket's evidence is the quoted output of steps 5 to 8. After step 8's commit, run
from a file as one `sh` script; `set -eu` stops it at the first command that fails. No
pipeline carries a result: each command writes to a file and its exit status is checked.

```sh
set -eu
base='<the full commit id step 1 records>'
readme=.scratch/quint-studio/README.md
out=ai_tmp/qs06
git rev-parse HEAD
git merge-base --is-ancestor "$base" HEAD
git diff --name-only "$base"...HEAD >| "$out/changed.txt"
printf '%s\n' .scratch/quint-studio/QS06-guest-image.md "$readme" >| "$out/expected.txt"
cmp "$out/changed.txt" "$out/expected.txt"
git show "${base}:$readme" >| "$out/base.md"
git show "HEAD:$readme" >| "$out/head.md"
sed 's/^\(| QS06 | .* | \)todo |$/\1done |/' "$out/base.md" >| "$out/want.md"
grep '^| QS06 | .* | done |$' "$out/want.md"
cmp "$out/want.md" "$out/head.md"
just check
status=$(git status --short)
test -z "$status"
```

The first `cmp` fails unless `git diff --name-only` lists exactly the two paths in Files
touched, in Git's order. `sed` writes the base's README with the Status cell of QS06's
Index row turned from `todo` to `done` and nothing else changed; `grep` prints that row
and fails if `sed` changed none; and the second `cmp` fails unless the committed README
is exactly that file. `base=` is filled in with the commit step 1 records.

## Open points

- Whether the git wrapper should also go into the image. Here it does not: it stays in
  QS04's provisioning, and the question is the later ticket's.
- Whether digests that differ between the two builds make the answer to "built" a no.
  The hand-back records the difference and says which it gave, and the maintainer rules.
- Who builds and publishes the image, and how it moves when the base or the snapshot
  moves: the later ticket's design, if the answers are yes.

## Hand-back notes

Empty until the ticket runs. Sections, in order: Base and branch; Baseline; The facts
and the amendments; Ruling and authorisations; The answers, built and hosted; Claims;
What was built or written outside the worktree, and its removal; What left the machine;
Checkpoint records; Departures; Verification; Open points.
