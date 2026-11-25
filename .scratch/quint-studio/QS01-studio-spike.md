---
id: QS01
title: "Spike: Quint Studio serve in a smolvm guest, by hand"
status: todo
depends_on: []
parallel_with: []
branch: ticket/qs01-studio-spike
estimated_size: M
---

# QS01: Spike: Quint Studio `serve` in a smolvm guest, by hand

## Context

Decision 0021, `docs/decisions/0021-project-managed-quint-cli.md`, says Quint Studio
"stays outside the repository" and that "nothing about it can be pinned or kept inside a
worktree". `docs/how-to/develop-locally.md`, "Optional: Quint Studio", says the same to
contributors. The tickets under `.scratch/quint-studio/` reverse that. They run Studio
inside a smolvm microVM that this repository manages, held the way AP02 holds Apalache:
pinned by checksum, installed into the worktree, never written to the home directory or
the global `PATH`, and reached only through a guarded recipe.

Before any recipe is written, this spike finds out whether that works at all. It changes
no tracked file except this ticket. Everything it builds lives in `ai_tmp/qs01/` and is
thrown away. Its hand-back notes are the deliverable: QS02 to QS05 cite them by claim
number, so every fact a later ticket needs must be written there, not left in `ai_tmp/`.

The facts below were read from upstream documentation on 2026-11-24 and not executed
here. Each one is a claim this ticket checks, and none may be copied into a later ticket
as settled until the hand-back records it.

- **Quint Studio** is a desktop application with an embedded server.
  `quint-studio serve` runs it headless. It serves the UI at
  `http://127.0.0.1:7000/?token=<token>` and an MCP endpoint on port 7001, which takes
  no token. On Linux it keeps its data in `~/.local/share/co.quint.studio/projects/`. It
  needs a Quint account, signed in with Google, and an AI provider. The Linux build is
  an x86_64-only AppImage. Downloads are offered as `stable/latest` only, with no version
  in the URL and no published checksum. Its `install.sh` notes that an AppImage needs
  FUSE unless it is extracted first.
- **smolvm v1.37.4** runs Linux guests booted from OCI images on libkrun, which uses
  Hypervisor.framework on macOS. On Apple Silicon, `--rosetta` runs x86_64 userspace in
  an arm64 guest; without it an x86_64 binary fails with `Exec format error`. The
  documentation describes these features:
  - `-v host:guest[:ro]` mounts a host directory over virtio-fs.
  - `-p` publishes a guest port on host loopback. The guest process must bind `0.0.0.0`.
  - Networking is off by default. `--block-private` and `--deny-cidr 127.0.0.0/8` keep
    the guest away from host loopback and private ranges.
  - A Smolfile, in TOML, describes a machine.
  - Each release tarball ships with `checksums.sha256`. smolvm is not on conda-forge.
  - Re-signing the binary drops the Hypervisor entitlement, so it must never be
    re-signed.

The maintainer ruled on four points on 2026-11-24. Every QS ticket holds them:

1. **Studio is pinned by its observed SHA-256.** Upstream publishes neither a version
   in the URL nor a checksum, so the pin is the hash of the build the maintainer
   reviewed. A new upstream build fails the install until someone re-pins it. This
   detects drift; it does not make installs reproducible.
2. **The worktree is mounted read-write.** The guest has open egress, but host loopback
   and private ranges are blocked.
3. **Agents are wired to Studio's MCP endpoint** in a ticket of their own, QS05.
4. **The only supported host is macOS on arm64.** Linux hosts are recorded as untested.

Read these:

- `docs/decisions/0021-project-managed-quint-cli.md`, all of it, and
  `docs/decisions/0005-project-managed-allium-cli.md`.
- `.scratch/apalache/AP02-checksummed-apalache.md`, "Design proposed in this ticket",
  for the pin file, the stamp and the installer pattern the later tickets copy.
- `.scratch/apalache/AP01-java-and-verify.md`, "Adversarial review", for the review
  command this ticket uses.
- `docs/how-to/develop-locally.md`, "Optional: Quint Studio".
- `tickets/CONVENTIONS.md` §11, §13 and §14.
- `.scratch/quint-studio/README.md`.

## Goal

A go or no-go, with evidence, for running Quint Studio's `serve` inside a smolvm guest on
the maintainer's macOS arm64 machine. Go means every claim below holds, or fails in a way
the hand-back names a workaround for. No-go means at least one claim fails with no
workaround the four rulings allow. On no-go the ticket stops, hands back, and records the
fallbacks: for example the native `.dmg` installed outside any VM, as decision 0021 has
it today.

Either way, the hand-back carries every name, path, flag, digest and hash QS02 to QS05
need, each quoted from a command's output and headed by its commit (§11).

## Claims this ticket checks

Later tickets cite these as "QS01 claim N". Each row's outcome goes under Hand-back
notes, "Claims", as holds, fails, or holds with a stated change.

| # | Claim | Check |
| --- | --- | --- |
| 1 | The v1.37.4 release has a darwin-arm64 tarball, its `checksums.sha256` names it, and the SHA-256 computed locally agrees. The binary, as unpacked, carries the `com.apple.security.hypervisor` entitlement and passes `codesign --verify` | step 2: the asset list, `shasum -a 256`, `codesign -d --entitlements - <binary>` and `codesign --verify --strict <binary>`, all read-only |
| 2 | Where smolvm keeps machine state, images and volumes on the host, and whether a flag or environment variable moves all of it into a directory inside the worktree | step 3: `--help`, the upstream documentation, and a run with the redirect, then step 3's probe for anything written or changed after a marker file |
| 3 | A digest-pinned `linux/amd64` image boots under `--rosetta` in an arm64 guest, and a static x86_64 binary in it runs. Whether Rosetta must be installed on the host first | step 4 |
| 4 | Studio's AppImage runs `serve` with no FUSE and no display. Where the extraction happens (on the host, which cannot run the x86_64 AppImage, or in the guest with `--appimage-extract` or `APPIMAGE_EXTRACT_AND_RUN=1`), and which guest libraries it needs beyond the image's own | step 5 |
| 5 | `serve`, bound to `0.0.0.0` in the guest and published with `-p 127.0.0.1:7000`, serves the UI to the host browser, and where the `?token=` value can be read from: a log line, a file or a command. The exact HTTP status, and the `Location` of any redirect, that a request without the token receives, and the exact shape of the token URL. That answer is a refusal: 401 or 403, or a redirect whose target is on `127.0.0.1` and is not the UI itself. A 200, a 5xx, or a redirect elsewhere fails the claim | step 6 |
| 6 | How sign-in works headless: `quint-studio login`, the browser through the forwarded port, or a device code. The maintainer performs it | step 7, a maintainer stop |
| 7 | A worktree's `.git` is a file that points outside the worktree. What git inside the guest needs to read the repository: the common git directory mounted read-only at its host path, or nothing. Whether Studio ever writes to git | step 8 |
| 8 | Studio's data, its projects and its sign-in, survive `stop` and `start` on a smolvm volume. Every path Studio writes, in the guest and inside the mounted worktree | step 9 |
| 9 | With `--block-private` and `--deny-cidr 127.0.0.0/8`, sign-in, the AI provider and DNS still work, and the guest cannot reach a listener on host loopback or on the host's LAN address | step 10 |
| 10 | What `quint-studio connect claude` writes, and where. Inspected only, never applied | step 11 |
| 11 | Studio under Rosetta is fast enough to use: start-up time, and the latency of one UI action and one MCP call | step 12 |
| 12 | `quint-studio --version` prints a version string in the guest, and the string QS03's pin file can record | step 5 |
| 13 | A read-only mount, or an empty one, at a subdirectory of the read-write worktree mount masks what lies beneath it, so the guest cannot write the gitignored paths the host runs or loads: `.tools/`, `.pixi/`, `.quint/`, `tools/quint/node_modules/`, `target/`, `fuzz/target/`, `loadgen/target/`, `.claude/`, `.codex/`, `ai_tmp/` and the machine state of claim 2. Whether a single file, not only a directory, can be masked the same way. Whether the guest can rename or replace a writable ancestor of a nested mask (`tools/quint/` or `tools/` for `tools/quint/node_modules/`, `fuzz/` and `loadgen/` for their `target/`), so that the host's path no longer leads to the masked tree. What the guest sees at each of those paths | step 9 |
| 14 | smolvm v1.37.4 has no conda-forge package that `pixi.lock` could hold, and no crates.io crate that `tools.txt` could install through cargo-binstall (decision 0011). If either exists, whether what it installs keeps the signed binary and its entitlement | step 2: each registry's HTTP API, with a positive control for a package each does carry |
| 15 | Studio's MCP endpoint on port 7001 answers an MCP `initialize` that carries no token or other credential, and neither `serve --help`, `connect claude --help` nor the documentation names an option that makes it require one | step 11 |
| 16 | How smolvm runs a command in a machine before `serve` starts (an exec into a running machine, or a boot command the host passes), how a volume is named per machine and listed, and whether smolvm leaves alone a file the host writes inside claim 2's state directory across `stop`, `start` and machine deletion | steps 4 and 9, from `--help` and a run of each |

## Non-goals

- No tracked file changes except this ticket and `.scratch/quint-studio/README.md`'s
  Status cell. No recipe, script, pin file, page or decision. QS02 to QS05 write those.
- No install outside the worktree beyond what an "Authorisation required" stop allows,
  and each one is recorded with how to remove it.
- No `quint-studio connect` applied to any agent's configuration.
- No Linux host. Ruling 4 records Linux as untested.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review" (T37, which
corrects §14's command, is still todo). `<id>` is `QS01`. There are four checkpoints,
each with a fixed number, so that `ai_tmp/review/QS01-<n>/` never changes meaning:

1. In step 3, before its "Authorisation required" stop, over what the stop will ask for.
2. In step 4, before its "Authorisation required" stop, over the Rosetta install.
3. At the end of step 5, over the choices steps 6 to 12 build on: where the state
   lives, the image and its digest, the extraction method and the guest libraries.
4. At step 13, over the hand-back notes before the final commit. Its artefact is the
   notes themselves, with the quoted output.

Checkpoints 1 and 2 run only if their stop is reached; when step 3 asks for both at once,
checkpoint 1 covers both and checkpoint 2 does not run. The hand-back records each
checkpoint that did not run, and why. No brief gives conclusions beyond what the
artefact quotes.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `.scratch/quint-studio/QS01-studio-spike.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS01's Status cell |

Everything else lives in `ai_tmp/qs01/`, which Git ignores.

## Steps

Each step quotes its commands' output and exit status, headed by the commit (§11). A
token, a cookie, an API key or an account identifier is redacted as `<redacted>` before
anything is quoted; the hand-back never holds a credential.

1. **Worktree and baseline.** Create the worktree on `ticket/qs01-studio-spike` from a
   base that holds the commit adding `.scratch/quint-studio/`, and record the base and
   its full commit id. Run `just initialize` (network, authorised). Run `just check` on
   the untouched worktree and quote its last line. Confirm the host: `uname -m` prints
   `arm64` and `sw_vers` names the macOS version. Show that the worktree lies under the
   home directory, `case "$PWD/" in "$HOME"/*) echo inside;; *) echo outside;; esac`,
   since step 3's probe takes its positive control from `ai_tmp/qs01/`; if it prints
   `outside`, stop and hand back.
2. **smolvm, read and fetched** (network, authorised). Read the v1.37.4 release's asset
   list. Download the darwin-arm64 tarball and `checksums.sha256` into a new, empty
   `ai_tmp/qs01/smolvm-download/`. Compute the SHA-256 with `shasum -a 256`, and compare
   it with the published line. Unpack into a new, empty `ai_tmp/qs01/smolvm/`, list the
   tree, and record every file the binary needs beside it (libkrun and its firmware, if
   shipped separately). Run the read-only `codesign` checks of claim 1. Never run
   `codesign --sign`, `codesign --remove-signature` or `xattr -c` on any file: re-signing
   drops the entitlement. Record `smolvm --version`, its exact output line, which QS02's
   Verification matches whole, and `smolvm --help`, and the help of every subcommand
   later steps use. Then claim 14, through the registries' HTTP APIs, so that no package
   tool writes a cache outside the worktree: `curl -sS -o <file> -w '%{http_code}\n'`,
   each body saved under `ai_tmp/qs01/`, for `https://crates.io/api/v1/crates/smolvm`
   and `https://api.anaconda.org/package/conda-forge/smolvm`, each beside the same
   request for a package it does carry (`cargo-hack` on crates.io, `just` on
   conda-forge) as the positive control, which must answer 200. Quote each status. A
   package counts as absent only on a 404 whose body names absence (§11, "Checks fail
   closed"); any other answer stops the step.
3. **Where state lives.** Before the first boot, find every place smolvm writes on the
   host: read `--help`, the documentation and, if needed, the source at the v1.37.4 tag.
   Look for an option or variable that moves machine state, the image cache and volumes
   into a given directory, and point it at `ai_tmp/qs01/state/`. Also check, read-only,
   whether Rosetta is installed: `pkgutil --pkg-info com.apple.pkg.RosettaUpdateAuto`,
   quoting its exit status.

   **The probe**, which later tickets reuse as "QS01 step 3's probe". Create
   `ai_tmp/qs01/marker` before the first boot. Afterwards run, over the roots `$HOME`,
   `$TMPDIR`, `$(getconf DARWIN_USER_CACHE_DIR)`, `/private/tmp`, `/Library` and
   `/usr/local`, excluding nothing, since `~/Library/Caches` is a likely home for an image
   cache:

   ```sh
   find <roots> \( -newer ai_tmp/qs01/marker -o -cnewer ai_tmp/qs01/marker \) -print \
     >| ai_tmp/qs01/probe.txt 2>| ai_tmp/qs01/probe.err
   ```

   The `-cnewer` arm sees a file whose modification time was set back, as an extracted
   image layer's is. Quote the exit status and every line of `probe.err`: each
   unreadable directory is recorded as unprobed, not as unchanged. Its positive
   controls, made after the marker and before the run: `ai_tmp/qs01/control-new`, a new
   file, and `ai_tmp/qs01/control-old`, written and then back-dated with
   `touch -t 200001010000`. Both lie under `$HOME`, as step 1 showed, and both must
   appear; no control file is written outside the worktree. Each other root must list
   at least one entry under `find <root> -maxdepth 1`, which shows the probe can read
   it. What the probe cannot see: a write outside its roots, a file deleted from a
   directory it does list (only the directory appears), and anything under a directory
   it could not read. The hand-back says so beside every result.

   If any smolvm state would land outside the worktree, or Rosetta is missing:

   **Adversarial review** (checkpoint 1: conformance and breakage, over the list the
   stop will ask for: each path outside the worktree, the command that writes it, why
   the spike needs it, and the command that removes it; and the Rosetta install, if the
   `pkgutil` check failed). Questions:
   - Could a flag or a variable keep the path inside the worktree after all?
   - Does each removal command remove exactly that path, and nothing beside it?
   - Is anything left on the host that step 14 cannot remove?

   **Authorisation required.** Stop before that first boot, name every path it will
   write, and the Rosetta install if it is needed, and ask the maintainer. If they allow
   it, record each path and the command that removes it, and remove it at step 14.
4. **Rosetta and the image** (network, authorised: the image pull). Choose a small
   `linux/amd64` base image whose C library and package set suit an AppImage (a Debian
   or Ubuntu slim image is the obvious candidate), and pin it by its digest, read from
   the registry, never by tag alone. Boot it with `--rosetta` and run `uname -m` and a
   static x86_64 binary in it. Boot it once without `--rosetta` and quote the
   `Exec format error`. Record, from `--help` and one run each, how smolvm runs a
   command in a machine before any service starts: an exec into a running machine, or a
   boot command the host passes (claim 16). If `--rosetta` fails because Rosetta is not
   installed, and step 3's stop did not already cover it:

   **Adversarial review** (checkpoint 2: conformance and breakage, over the error, the
   install command and what it leaves on the host). Questions: is Rosetta what the error
   needs, and can step 14 remove it, or must the hand-back name it as remaining?

   **Authorisation required.** Installing Rosetta (`softwareupdate --install-rosetta`)
   changes the host outside the worktree. Stop and ask; the maintainer may run it.
5. **Studio's AppImage** (network, authorised). Read `install.sh` without running it, and
   record the exact `stable/latest` URL for the Linux x86_64 AppImage and every redirect
   it follows (`curl -fsSIL`). Download the AppImage into a new, empty
   `ai_tmp/qs01/studio-download/`, record its SHA-256 and the final URL. Then find the
   way to run it without FUSE, in this order, and record which worked:
   - in the guest, `APPIMAGE_EXTRACT_AND_RUN=1` with the AppImage mounted read-only;
   - in the guest, `--appimage-extract` into a guest directory or a volume;
   - on the host, an `unsquashfs` of the embedded filesystem at its offset, which needs
     a tool from conda-forge's `squashfs-tools`, and only if neither guest method works.

   Run `quint-studio --version` and `quint-studio serve --help` in the guest (claim 12).
   Every library `serve` fails to find is installed in the guest and recorded with its
   package name and version; say whether the chosen image could carry them instead, and
   whether a later ticket would need an image of its own or a pinned package snapshot.

   **Adversarial review** (checkpoint 3: conformance and breakage, over steps 3 to 5's
   quoted output and the choices they made, before step 6 builds on them). Questions:
   - Does the state location, the image digest or the extraction method rest on output
     the step quotes, or on the upstream text in Context?
   - Could the extraction method leave a copy of Studio that a later boot runs after the
     AppImage has changed?
   - Is every guest library recorded with a version a later ticket can pin?
6. **The UI on host loopback.** Run `serve` bound to `0.0.0.0:7000` in the guest with
   `-p 127.0.0.1:7000:7000`, and the worktree mounted read-write at its own host path.
   From the host, run
   `curl -sS -o /dev/null -w '%{http_code} %{redirect_url}\n' http://127.0.0.1:7000/`
   and the same with the token, quoting each exit status: record the exact status, and
   any redirect's target, that the request without the token receives (claim 5), since
   QS04's Verification requires that exact answer and nothing else. If it is not a
   refusal as claim 5 defines one, claim 5 fails and the hand-back says so; it is never
   recorded as the expected answer. Open the token URL
   in the host browser. Record where the token came from, and the URL's shape with the
   token redacted. Show that the port is not bound on any other host address:
   `lsof -nP -iTCP:7000 -sTCP:LISTEN`. Have the guest create a file in the mounted
   worktree, as the guest's user, and record its owner and mode on the host
   (`ls -ln`).
7. **Sign-in.** **Stop for the maintainer.** Sign-in needs their Google account and an AI
   provider's key, which only they hold. Record the method that worked and every prompt it
   showed, with identifiers redacted, and which guest files changed (step 3's predicate,
   `-newer` or `-cnewer` a guest marker, over the guest's home and `/tmp`, with the same
   two controls made in the guest). Never print the files' contents.
8. **Git in the guest.** Read the worktree's `.git` file and the `commondir` it leads to.
   Show `git -C <worktree> status` failing in the guest with only the worktree mounted.
   Mount the common git directory read-only at its host path and show `git status` and
   `git log -1` working, and `git commit --allow-empty` failing on the read-only mount.
   Open the worktree as a Studio project, and record whether Studio runs git at all and
   whether it ever writes.
9. **Persistence and writes.** Put Studio's data directory, and any other path step 7
   found, on a smolvm volume. `stop` and `start` the machine, and show that Studio is
   still signed in and the project is still listed. Record how the volume is named, how
   it is listed, and whether a second machine can be given a volume of its own name
   (claim 16); write a file into the state directory from the host, and show whether it
   survives `stop`, `start` and the machine's deletion (claim 16). Record every path
   Studio wrote inside the mounted worktree, in two views:
   `git status --short --ignored`, before and after a Studio session, for what Git sees;
   and step 3's predicate over the whole worktree, excluding nothing, against a marker
   made just before the session, with step 3's two controls, for writes inside ignored
   directories and to files that already existed, which the status view cannot show. Any
   such path is a design input for QS04, since `just check` fails on a worktree it
   changed. Then claim 13: the read-write worktree mount exposes every gitignored path
   under it, which no diff shows, and the host runs or loads some of them: `.tools/` and
   `.pixi/envs/default/bin`, first on every recipe's `PATH`; `.quint/` and
   `tools/quint/node_modules/`, which `just quint` runs; the `target/` directories,
   whose binaries the tests run; `.claude/` and `.codex/`, whose ignored local settings
   the agents load; and `ai_tmp/`, where verification scripts are kept and run. Mount an
   empty directory, or the same path read-only, over each of those directories and over
   the state directory; `.claude/` and `.codex/` also hold tracked files, so theirs is
   the same path read-only. To show whether a single file can be masked, mount a new,
   untracked `qs01-mask-probe` at the worktree root read-only over itself, and remove it
   after. Show from the guest whether a write at each fails and what the guest lists,
   and from the host that each host copy is unchanged. Then, with the nested masks in
   place, try from the guest to rename each writable ancestor of a nested mask
   (`tools/quint/`, `tools/`, `fuzz/`, `loadgen/`) to a new name and back, quoting each
   exit status, and show from the host that the masked tree is still the one the host
   path leads to; restore anything that moved. If masking does not work, or an ancestor
   can be renamed, the read-write ruling and state kept in the worktree conflict, and
   the hand-back says so for the maintainer.
10. **Egress, loopback and private ranges.** From the host, start a listener on
    `127.0.0.1` and one on the host's LAN address. First boot with networking on and
    neither deny flag, and show the guest reaching both listeners, recording the
    guest-side address that reaches host loopback: that boot is the control, and without
    it an unreachable listener proves nothing. Then boot with `--block-private` and
    `--deny-cidr 127.0.0.0/8`, and show the same probes, to the same addresses, failing,
    and an HTTPS request to a public host succeeding. Repeat sign-in's check
    and one AI-provider call. Record the guest's DNS resolver address, and whether
    blocking private ranges blocks it.
11. **What `connect claude` would write.** Run `quint-studio connect claude --help`, and
    find whether a dry-run mode exists. If it does not, run `connect claude` in the
    guest with `HOME` pointed at an empty guest directory, and record every file written
    and its content, with any credential redacted. Record the MCP endpoint's URL, its
    path and its transport. Never apply the result on the host. Then claim 15: from the
    guest, send the endpoint an MCP `initialize` with no token, cookie or credential
    header and quote the answer, and quote every option in `serve --help`,
    `connect claude --help` and the documentation that names a token, a key or
    authentication, or that none does.
12. **Performance.** Record wall-clock start-up from boot to the UI answering, the
    latency of one UI action and of one MCP `tools/list` call, each three times. The
    maintainer judges whether it is tolerable, and the hand-back records their verdict.
13. **Adversarial review** (checkpoint 4: conformance and breakage, over the hand-back
    notes, drafted now in full except the clean-up's output and Verification, which
    step 14 adds, before the final commit). Questions:
    - Could any claim read "holds" while its quoted output shows something weaker: a
      skipped control, an exit status not shown, a step run on another machine?
    - Does the hand-back carry every fact QS02 to QS05 will need, or does one survive
      only in `ai_tmp/`?
    - Does any quoted output carry a token, a cookie, a key or an account identifier?
    - Is anything left on the host outside the worktree that step 14 does not remove?
14. **Clean up and hand back.** Stop and delete every machine and volume the spike made.
    Remove every path outside the worktree that step 3 or 4 was allowed to write, and
    show the removal: `test ! -e` on each path step 3 recorded, quoting each exit
    status, which is the evidence of removal. Then rerun step 3's probe against the same
    marker, with its controls, and list every line outside the worktree that the first
    run lacked; each is either the parent of a removed path or named as activity that is
    not the spike's, with the reason, and the hand-back says which. Add the clean-up's
    output to the notes step 13 reviewed, changing nothing else in them; a change to a
    claim's outcome or a fact after step 13 needs the checkpoint again. Set
    `status: done` here and in `.scratch/quint-studio/README.md`, and commit. Run
    Verification on that commit, quote it in the notes' Verification section headed by
    the full commit id, and commit the record. On the record commit, run
    `git diff --name-only HEAD~1 HEAD`, which must list only this ticket, then
    `just check` and `git status --short`, and report their output to the maintainer
    with the hand-back, headed by that commit, since a commit cannot quote its own
    check. If any fails, fix the cause in a new commit and run all three again.
    **Stop** before pushing or opening a pull request: both need authorisation
    (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- Every claim has an outcome, backed by quoted output headed by its commit.
- The hand-back records, for QS02: the release's asset name, its published and computed
  SHA-256, the files the binary needs beside it, its entitlement, the exact
  `smolvm --version` line, the state paths and whether they can be redirected, whether
  conda-forge or crates.io carries smolvm (claim 14), and the flags used for mounts,
  ports and networking.
- The hand-back records, for QS03: the `stable/latest` URL, its final URL, the AppImage's
  SHA-256 on the day, its `--version` string, and the extraction method that worked.
- The hand-back records, for QS04: the image reference with its digest, any guest
  packages and their versions, Studio's data paths, the paths Studio writes in the
  worktree, in both of step 9's views, the git mount that works, whether masking mounts
  work for directories and for a single file (claim 13), the guest-side address that
  reached host loopback in the control boot, how the token is read, the URL's shape and
  the exact answer without the token (claim 5), how a command runs before `serve`, how
  volumes are named and listed, and whether a host file in the state directory survives
  (claim 16), and the Smolfile keys upstream documents for each of these, quoted from
  `--help` or the documentation.
- The hand-back records, for QS05: the MCP endpoint's URL and transport, what
  `connect claude` writes, and whether the endpoint takes or can require a credential
  (claim 15).
- No quoted output holds a credential.
- Nothing the spike wrote outside the worktree remains, or what remains is named with
  the maintainer's permission.
- Each checkpoint that ran is recorded as §14's Record asks, with AP01's deltas, and
  each that did not run is named with the reason.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record
  commit, as step 14 reports.

## Verification

The spike's evidence is the quoted output of steps 2 to 12. After the commit, run from a
file as one `sh` script; `set -eu` stops it at the first command that fails:

```sh
set -eu
base='<the full commit id step 1 records>'
git rev-parse HEAD
git diff --name-only "$base"...HEAD >| ai_tmp/qs01/changed.txt
printf '%s\n' .scratch/quint-studio/QS01-studio-spike.md .scratch/quint-studio/README.md \
  >| ai_tmp/qs01/expected.txt
cmp ai_tmp/qs01/changed.txt ai_tmp/qs01/expected.txt
just check
status=$(git status --short)
test -z "$status"
```

`cmp` fails unless `git diff --name-only` lists exactly the two paths in Files touched,
in Git's order. The `base=` line is filled in with the commit step 1 records before the
script runs.

## Hand-back notes

Empty until the ticket runs. Sections, in order: Base and branch; Baseline; Go or no-go;
Claims; Facts for QS02; Facts for QS03; Facts for QS04; Facts for QS05; What was written
outside the worktree, and its removal; Checkpoint records, one per checkpoint, 1 to 4,
each saying whether it ran; Departures; Verification; Open points.
