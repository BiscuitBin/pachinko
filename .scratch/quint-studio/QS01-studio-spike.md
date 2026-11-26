---
id: QS01
title: "Spike: Quint Studio serve in a smolvm guest, by hand"
status: done
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
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review" (T37,
which corrects §14's command, is still todo). One delta overrides them: the reviewer is
Codex Sol, `-m gpt-6.2-sol`, in place of the `-m gpt-6-astra` that AP01's command and
§14 print, by the maintainer's choice of 2026-11-25. AP01's fallback to a fresh-context
`claude-opus-5-5` subagent still applies, and the hand-back notes record the model that
ran. `<id>` is `QS01`. There are four checkpoints, each with a fixed number, so that
`ai_tmp/review/QS01-<n>/` never changes meaning:

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

Every quote below ran on 2026-11-25 on the maintainer's machine (`uname -m`
`arm64`, macOS 26.4 build 25E246, Apple M5 Pro) at `c5ca91f` with no tracked change
except this ticket's own notes, unless its heading says otherwise. In the quoted `lsof`
lines the account name in the USER column is written `<user>`; absolute paths are left
as the commands printed them, as this repository's other tickets do.

### 1. Base and branch

Base `quint-studio-smolmachine` at `c5ca91f186dcffd14c7e372529c7e8696195b40f`, the branch
that holds both AP02 (`ffe512b`) and `.scratch/quint-studio/`. The work ran on
`ticket/qs01-studio-spike` in its own worktree,
`/Users/scutting/.supacode/repos/pachinko/QS01`, made with
`supacode repo worktree-new --branch ticket/qs01-studio-spike --base quint-studio-smolmachine --name QS01`
(exit 0). The worktree lies under the home directory:

```text
$ case "$PWD/" in "$HOME"/*) echo inside;; *) echo outside;; esac
inside
```

### 2. Baseline

`just initialize` exited 0 and ended `Ready. Next: just check.`; as a secondary worktree
it skipped `install-hooks`. At `c5ca91f`, `just check` on the untouched worktree, exit
0, and `git status --short` printed nothing (exit 0):

```text
All checks passed and the worktree is unchanged.
```

### 3. Go or no-go

**Go, conditional.** The mechanism works as measured. Studio 0.2.11's `serve` ran in a
smolvm v1.37.4 guest on this machine: UI on host loopback behind its token, signed in,
two providers, one scan with 92 model calls, all with private ranges and host loopback
denied, state inside the worktree, and no write outside the worktree by smolvm that the
checks could find.

The condition is claim 13's conflict. Under ruling 2 as it stands, a secondary worktree
has no protection for its `.git` pointer that was tested in place: the pointer is a
file inside the read-write mount, a file cannot be masked, and the only workaround
found, `chflags uchg`, was tried on a throwaway file, never on `.git`, and removal by
the guest was not tested. So QS04 goes ahead only if the maintainer accepts one of
three things (Open point 1): the `chflags uchg` workaround, with those limits; Studio
from the primary checkout only; or a change to ruling 2, the worktree read-only with
writable subdirectories.

If none is accepted, that part of claim 13 fails with no workaround the rulings allow,
and by the Goal's terms the outcome is no-go. QS04 and QS05 do not run, and the
fallback is the one the Goal names, as decision 0021 has it today: the native `.dmg`
installed outside any VM, outside the repository. The two pins, smolvm in QS02 and the
AppImage in QS03, do not depend on that ruling. QS02's decision 0022 and its demo boot
do, since both carry the read-write mount of ruling 2. The README of these tickets has
QS02 to QS05 not start on a no-go, so whether the two pins go ahead in that case is
the maintainer's to say. Open points 2 to 5 and 14 also reshape the later tickets and
are the maintainer's to rule on before QS02 starts.

On 2026-11-25, before the sign-in steps and with claim 13's conflict put to them, the
maintainer chose "Go on (Recommended)": to go on with the spike's remaining steps. That
was leave to finish the spike, not a ruling on Open point 1.

Claim 12 fails and has a workaround. Claim 13 fails in two of its parts and has a
workaround for one of them. Claims 2, 3, 4, 5, 7, 8, 10, 11, 15 and 16 hold only with
the changes they state.

### 4. Claims

1. **Holds.** The v1.37.4 release
   (`https://api.github.com/repos/smol-machines/smolvm/releases/tags/v1.37.4`, HTTP 200,
   published `2026-11-25T07:18:08Z`) lists five assets: `checksums.sha256`,
   `smolvm-1.37.4-darwin-arm64.tar.gz` (46449605 bytes),
   `smolvm-1.37.4-linux-arm64.tar.gz`, `smolvm-1.37.4-linux-x86_64.tar.gz` and
   `smolvm-1.37.4-windows-x86_64.zip`. The tarball's line in `checksums.sha256`, GitHub's
   asset digest and the local hash agree:

   ```text
   $ grep ' smolvm-1.37.4-darwin-arm64.tar.gz$' checksums.sha256
   631be08a4e2cd557a1d47963b2d134e7d65a4b15966751730f0553df16f515e0  smolvm-1.37.4-darwin-arm64.tar.gz
   $ shasum -a 256 ai_tmp/qs01/smolvm-download/smolvm-1.37.4-darwin-arm64.tar.gz
   631be08a4e2cd557a1d47963b2d134e7d65a4b15966751730f0553df16f515e0  ai_tmp/qs01/smolvm-download/smolvm-1.37.4-darwin-arm64.tar.gz
   shasum exit 0
   $ grep ' smolvm-1.37.4-darwin-arm64.tar.gz$' checksums.sha256 | shasum -a 256 -c -
   smolvm-1.37.4-darwin-arm64.tar.gz: OK
   shasum -c exit 0
   ```

   `checksums.sha256` itself hashes to
   `071f4add678053d36a8e70d96414694426751a839ed348695b5495e16090cfee`, which is also the
   digest GitHub lists for it. The binary as unpacked, `smolvm-bin`, read-only:

   ```text
   $ codesign -d --entitlements - smolvm-bin
   [Dict]
       [Key] com.apple.security.cs.allow-jit
       [Value]
           [Bool] true
       [Key] com.apple.security.cs.disable-library-validation
       [Value]
           [Bool] true
       [Key] com.apple.security.hypervisor
       [Value]
           [Bool] true
   codesign -d exit 0
   $ codesign --verify --strict --verbose=2 smolvm-bin
   smolvm-bin: valid on disk
   smolvm-bin: satisfies its Designated Requirement
   codesign --verify exit 0
   ```

   With a stated qualification QS02 needs: the signature is ad hoc, not a Developer ID.
   `codesign -dvv` prints `Signature=adhoc`, `TeamIdentifier=not set` and
   `flags=0x2(adhoc)`, and `spctl -a -vv -t execute smolvm-bin` prints `rejected`, exit
   3. `codesign --verify` therefore proves the file is intact against its own embedded
   hashes and nothing about who built it; the release checksum is the only provenance.
   The six libraries under `lib/` each pass `codesign --verify --strict`, exit 0.
2. **Holds with a stated change**, for the database, the usage journal, the image cache,
   the registry token cache and each machine's disks and sockets. Volumes are step 9's
   (claim 16).

   *Where state lives by default.* Upstream's
   `docs/teardown/references/locations.md` at the tag names, on macOS,
   `$HOME/Library/Application Support/smolvm`, `$HOME/Library/Caches/smolvm` (with
   `vms/`, `image-seeds/`, `registry-tokens/`), four sibling caches
   (`smolvm-pack`, `smolvm-libs`, `smolvm-registry`, `smolvm-image-archives`) and
   `$HOME/.config/smolvm`. This host already held the first two before the spike ran,
   from another project's `smolmachines` 1.32.4 Python package, with live machines in
   them (section 9). The spike never ran against them.

   *The redirect.* `smolvm --help`, exit 0:

   ```text
         --state-dir <PATH>
             Keep all smolvm state (machines, images, caches, the database) under PATH instead of the user's default directories, for example to run a second engine version beside an installed one
   ```

   The same help's "Key Flags" table says `--state-dir` applies to
   `every command (Linux)`. It works on macOS: the source at the tag
   (`889ec606a08ad08eeb6e808e2358342a647a7f57`), `src/process.rs`,
   `apply_system_data_root`, is compiled for every Unix. It reads `SMOLVM_DATA_DIR`, which
   `--state-dir` sets, creates that directory, then sets `HOME` to it and removes
   `XDG_CACHE_HOME`, `XDG_DATA_HOME` and `XDG_CONFIG_HOME`, so every path above follows.
   Two side effects QS02 must design for: when `DOCKER_CONFIG` is unset and `~/.docker`
   exists, the function points `DOCKER_CONFIG` at the real `~/.docker`, so a pull would
   read the user's registry credentials; and `SMOLVM_USAGE_DIR` moves the usage journal
   on its own (`src/usage.rs`).

   *The change: the root must be given as a relative path in the environment.* With an
   absolute root inside this worktree the first boot failed:

   ```text
   $ SMOLVM_DATA_DIR=/Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp/qs01/state smolvm machine run --rosetta --image docker.io/library/debian@sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0 -- uname -m
   Starting ephemeral machine (vm-bb8a0621)...
   WARN VMM exited with code 1 pid=83346 cause=Exited
   Error: agent operation failed: start machine: agent operation failed: monitor agent: Error: agent operation failed: add vsock port: cannot address Unix socket /Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp/qs01/state/Library/Caches/smolvm/vms/c59f0552377d03eb/agent.sock: Operation not permitted (os error 1) at path "/tmp/ks-arDtZJ" (boot process exited (code 1) before the agent was ready)
   run exit 1
   ```

   A machine's sockets live at `<root>/Library/Caches/smolvm/vms/<16 hex>/<name>.sock`,
   44 bytes after the root plus the name (`agent.sock`, `control.sock`, `ports.sock`,
   `port-forward.sock`). macOS limits a socket path to 103 bytes. For a longer one,
   smolvm's `libkrun` (submodule commit `36b0846191bf8dbfc044c98cf43922c2f1f221e4`,
   `src/socket/src/lib.rs`, `SocketPath::new`) makes a mode-0700 directory
   `/tmp/ks-<6 random>` holding a symlink `d` to the socket's directory, and removes it
   when done. `/tmp` is hard-coded ("Ignore TMPDIR") and no variable moves it. The
   spike's profile denied that write, which is the `Operation not permitted` above. An
   absolute root inside any worktree of this repository is too long: the root may be at
   most 42 bytes, and this worktree's own path is 45.

   The length test is on the path string as given, before any canonicalisation. So the
   spike set `SMOLVM_DATA_DIR=ai_tmp/qs01/state`, relative, with the working directory
   at the worktree root, where `agent.sock` is 71 bytes and `port-forward.sock` 78.
   `--state-dir` cannot do this: `src/main.rs`, `state_dir_from_args`, makes its value
   absolute. With the relative variable, and every file write outside `ai_tmp/qs01/`
   denied by `sandbox-exec` (section 9 quotes the profile and its controls),
   `machine run --net --rosetta` from the index reference of claim 3, running
   `uname -m`, printed `x86_64` (`logs/boot3.txt`, 14:48:41 local, which ends at that
   line and holds no exit status). `machine data-dir --name qs01-a` then named
   `ai_tmp/qs01/state/Library/Caches/smolvm/vms/dda19bb99b10c1b8`; that line is in no
   saved log, and `logs/s9-find-after-start.txt` lines 10 to 14 list files under that
   directory. Both commands were run again later through the same wrapper, with each
   status saved. In the rerun the machine named is `qs01-s`, the one that still
   existed, and `<index digest>` stands for that same index reference; the log does
   not say which of the wrapper's two profiles was in force.
   `logs/rerun-statuses.txt` (18:17 local) lines 1 to 6, at `c5ca91f`, with
   uncommitted changes (this ticket's notes):

   ```text
   $ smolvm-qs01 machine run --net --rosetta --image <index digest> -- uname -m
   x86_64
   run exit 0
   $ smolvm-qs01 machine data-dir --name qs01-s
   ai_tmp/qs01/state/Library/Caches/smolvm/vms/a33492321423404d
   data-dir exit 0
   ```

   *What the run with the redirect wrote*, from step 3's probe (section 9): under
   `ai_tmp/qs01/state/`, in `Library/Application Support/smolvm/`, `server/smolvm.db`
   with its `-shm` and `-wal`, `server/smolvm.transfer-admission.lock`,
   `usage/events.jsonl` and `usage/install_id`; and in `Library/Caches/smolvm/`,
   `vms/<16 hex>/`, `image-seeds/<64 hex>/storage.raw`, `agent-rootfs-digests-v2/` and
   `registry-tokens/<64 hex>.json`. That last file is a registry pull token smolvm
   caches; its content was never read. It also wrote **beside its own binary**, on first
   use: `overlay-default.ext4` (10 GB apparent, 25M on disk) and `storage-template.ext4`
   (20 GB apparent, 10M on disk), unpacked from the `.zst` files shipped there. An
   install directory that is read-only, or whose content a stamp hashes whole, breaks on
   that. libkrun also makes `$TMPDIR/krun-eventfd-XXXXXX/e` and falls back to a pipe
   when it cannot (`src/utils/src/macos/eventfd.rs`); the spike pointed `TMPDIR` into
   `ai_tmp/qs01/tmp/`, which was empty after the boots.

   The first probe exited 1 and could not read 447 directories. Outside the worktree
   it lists two paths that name smolvm, both in the directory of a machine that was on
   this host before the spike. No check found a write by the spike's smolvm outside the
   worktree, which is weaker than showing there was none. Section 9 carries the probe's
   output, when it ran against the boots, the evidence for whose machine that is, and
   what the probe cannot see.
3. **Holds with a stated change.** The image is `debian:trixie-slim` (Debian 13.8,
   glibc 2.41). The boots quoted in this claim, and the first machine `qs01-a`, named it
   by the digest of its multi-platform index, read from the registry
   (`https://registry-1.docker.io/v2/library/debian/manifests/trixie-slim`). The digest
   to carry forward is the `linux/amd64` manifest's, not the index's (claim 16,
   Departure 6). The `curl` that fetched the index saved no exit status; the saved
   `HTTP/2 200` line and the body's hash, equal to the `docker-content-digest` header,
   stand in for it. At `c5ca91f`, with uncommitted changes (this ticket's notes), from
   the saved response:

   ```text
   $ sed -n '1p;5p' ai_tmp/qs01/api/debian-index.headers
   HTTP/2 200
   docker-content-digest: sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0
   sed exit 0
   $ shasum -a 256 ai_tmp/qs01/api/debian-index.json
   8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0  ai_tmp/qs01/api/debian-index.json
   shasum exit 0
   $ jq -r '.manifests[] | select(.platform.os == "linux" and (.platform.architecture == "amd64" or .platform.architecture == "arm64")) | "\(.platform.architecture) \(.platform.variant // "-") \(.digest)"' ai_tmp/qs01/api/debian-index.json
   amd64 - sha256:ab3b3c8c5159eebfca4376430132d53c4d425f869c765ecbe47177a030da575d
   arm64 v8 sha256:65d15c0ac2dd6639a130a05ffaf36b9d60697ef8d07125d79aecaaa9871bc97b
   jq exit 0
   ```

   `--rosetta` "Pulls `linux/amd64` unless `--oci-platform` says otherwise" (`--help`).
   In the quotes below `<ref>` is the index reference,
   `docker.io/library/debian@sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0`.
   The same command with and without the flag. The guest command ends in `echo`, so the
   machine's own status says only that it ran, and the saved logs (`logs/boot6.txt`,
   `logs/boot6--rosetta.txt`) hold no outer exit line. The AppImage's status is the
   `appimage exit 126` line:

   ```text
   $ smolvm-qs01 machine run --net --image <ref> -v <worktree>/ai_tmp/qs01/studio-download:/opt/studio-dl:ro -- sh -c 'uname -m; /opt/studio-dl/quint-studio.AppImage --appimage-version; echo "appimage exit $?"'
   aarch64
   appimage exit 126
   sh: 1: /opt/studio-dl/quint-studio.AppImage: Exec format error
   $ smolvm-qs01 machine run --net --rosetta --image <ref> -v <same> -- sh -c '<same>'
   x86_64
   appimage exit 126
   /usr/bin/sh: 1: /opt/studio-dl/quint-studio.AppImage: Exec format error
   ```

   Under `--rosetta` the guest's whole userland is x86_64 (`uname -m`, and the image's
   own `sh` ran). The change: **the AppImage itself still gets `Exec format error` under
   `--rosetta`.** Its ELF header carries the AppImage magic `41 49 02` at bytes 8 to 10
   (`od -A d -t x1 -N 20`: `7f 45 4c 46 02 01 01 00 41 49 02 00 00 00 00 00`), and the
   guest kernel's Rosetta registration does not match a header with those bytes set. Two
   ways run it, both exit 0 and both printing
   `AppImage runtime version: https://github.com/AppImage/type2-runtime/commit/fedfad5`:
   a copy with those three bytes zeroed, which is the static x86_64 binary the claim asks
   for, and the untouched file passed to the interpreter smolvm mounts in the guest,
   `/mnt/rosetta/rosetta /opt/studio-dl/quint-studio.AppImage --appimage-version`. The
   second keeps the pinned file byte-identical and is what the spike used.

   The output of those first runs was not saved. The saved transcript is a later rerun
   of them, `logs/transcripts-s45.txt`, written at 15:18 local by
   `ai_tmp/qs01/bin/guest-transcripts.sh` running in `qs01-a`. No file holds the host
   command that started the script, or that command's exit status; each guest command's
   status is in the transcript. Its `$` lines are the script's own labels, and the
   commands are the script's lines 5 to 13. At `c5ca91f`, with uncommitted changes
   (this ticket's notes), lines 1 to 3, 10 to 24 and 62 to 65:

   ```text
   $ uname -m
   x86_64
   exit 0
   $ od -A d -t x1 -N 20 /opt/studio-dl/quint-studio.AppImage
   0000000 7f 45 4c 46 02 01 01 00 41 49 02 00 00 00 00 00
   0000016 03 00 3e 00
   0000020
   exit 0
   $ /opt/studio-dl/quint-studio.AppImage --appimage-version   (direct)
   exit 126
   $ /mnt/rosetta/rosetta /opt/studio-dl/quint-studio.AppImage --appimage-version   (through the interpreter)
   exit 0
   $ cp /opt/studio-dl/quint-studio.AppImage /tmp/qs-noexec.AppImage; chmod 644; run through the interpreter
   -rw-r--r--
   exit 0
   $ a copy with bytes 8 to 10 zeroed, run directly (a static x86_64 binary under Rosetta)
   0000000 7f 45 4c 46 02 01 01 00 00 00 00 00
   exit 0
   /Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp/qs01/bin/guest-transcripts.sh: 9: /opt/studio-dl/quint-studio.AppImage: Exec format error
   AppImage runtime version: https://github.com/AppImage/type2-runtime/commit/fedfad5
   AppImage runtime version: https://github.com/AppImage/type2-runtime/commit/fedfad5
   AppImage runtime version: https://github.com/AppImage/type2-runtime/commit/fedfad5
   ```

   The last four lines are standard error, which the file holds at its end and not
   beside the commands that wrote it. So the three `AppImage runtime version` lines and
   the direct run's `Exec format error` are matched to the three runs that exited 0
   (the interpreter, the interpreter on a mode-0644 copy, and the zeroed copy) and the
   one that exited 126 by count, not by position.

   Rosetta was already installed on the host, so nothing was installed:
   `pkgutil --pkg-info com.apple.pkg.RosettaUpdateAuto` exited 0
   (`version: 1.0.0.0.1773981789`), and the file smolvm itself tests for
   (`src/platform/macos.rs:101,109`),
   `/Library/Apple/usr/libexec/oah/RosettaLinux/rosetta`, exists (`test -e`, exit 0).
   Without that file smolvm does not attach the Rosetta runtime
   (`src/agent/launcher_dynamic.rs:873`, `src/agent/launcher.rs:2463`,
   `src/vm/backend/libkrun.rs:209`, the last logging `Rosetta requested but not
   available - x86_64 binaries will not work`), but `--rosetta` still asks for the
   `linux/amd64` platform: `image_platform` takes the platform `--oci-platform` names
   and otherwise reads only the flag (`src/vm/rosetta.rs:59-61`, called at
   `src/cli/machine.rs:2396-2397`), and nothing else calls `rosetta::is_available` or
   `runtime_path`, so no command refuses first. What is asked for is not always what
   the machine gets: a named machine made from the index digest came up arm64 with
   both `--rosetta` and `--oci-platform linux/amd64` (claim 16). The source has a
   second path for a machine whose guest cannot pull, a fetch on the host that
   resolves an index for the host's own architecture
   (`src/image_store.rs:257-262,305-306,484`), and the guest agent refuses an archive
   of another architecture when translation is not active
   (`crates/smolvm-agent/src/storage.rs:2227-2239`). So on a host without Rosetta a
   machine may come up arm64, be refused at start, or hold an amd64 image with no
   translator and fail at its first exec, depending on the image reference and the
   path the pull takes. All of this is read from the source, not run: whether a host
   with no Rosetta fails as the Context says was not tested. QS02's preflight should
   test for the file, not the receipt, and refuse before it creates a machine.
4. **Holds with a stated change.** `serve` ran in the guest with no FUSE mount and no
   display, from a tree extracted in the guest. Neither extraction method mounts
   anything. The guest kernel does offer FUSE (`/dev/fuse` exists and `fuse` is in
   `/proc/filesystems`) and has no `fusermount`; whether the AppImage's own FUSE mount
   would work there was not tried. From `logs/transcripts-s45.txt` (claim 3 says how it
   was made), lines 25 to 35, at `c5ca91f`, with uncommitted changes (this ticket's
   notes):

   ```text
   $ FUSE in the guest
   crw-rw-rw- 1 root root 10, 229 Nov 25 23:14 /dev/fuse
   ls /dev/fuse exit 0
   3
   grep fuse /proc/filesystems exit 0
   command -v fusermount exit 127
   $ no display
   0
   grep DISPLAY exit 1 (1 = no match)
   1
   positive control grep PATH exit 0
   ```

   By the script's lines 14 and 15, `3` is `grep -c fuse /proc/filesystems`, `0` is
   `env | grep -c 'DISPLAY'` and `1` is its positive control, `env | grep -c '^PATH='`.
   The first `serve`, from the tree method 2 below extracted, on port 7000. No file
   holds its command line. As far as it can be reconstructed, it was a `machine exec`
   in `qs01-a` that ran, from `/tmp/ws`,
   `timeout 25 /opt/quint-studio/squashfs-root/AppRun serve --bind 0.0.0.0 --port 7000 --trust-folder /tmp/ws`
   and then printed the saved lines. That is a partial reconstruction, not a recipe
   to copy. The saved output, `logs/serve1.txt` lines 2 and 4 to 12:

   ```text
   serve (timeout 25) exit 124
   Trusted /tmp/ws
   [caps] no usable systemd user slice: systemctl --user --version failed
   [caps] cpu none, memory none (no systemd user manager)
   http://127.0.0.1:7000/?token=<redacted>
   listening on every interface, but this machine has no address on a network it routes out of — only the loopback URL above works here
   MCP server: http://127.0.0.1:40521/operator
   not signed in — run: quint-studio login
   [loops] project.open failed (missing) after 89.682708ms
   [loops] could not read what the loops landed: Studio needs git to open a project, and git does not run on this computer.
   ```

   Exit 124 is `timeout` ending a server that was still running. Both extraction
   methods reached `serve`: method 2 here, and method 1 on port 7005, below. The
   extraction methods, in the ticket's order:

   - `APPIMAGE_EXTRACT_AND_RUN=1`, AppImage mounted read-only: works, through the
     interpreter (`APPIMAGE_EXTRACT_AND_RUN=1 /mnt/rosetta/rosetta <AppImage> <args>`).
     It unpacks to `/tmp/appimage_extracted_e12087593f6db4dfc5ccc13ac3f3406c/`, where the
     suffix is the AppImage's MD5 (and upstream's `etag`), and runs `AppRun` from there.
     It reaches `serve` too. The command is the script's line 22,
     `APPIMAGE_EXTRACT_AND_RUN=1 timeout 20 /mnt/rosetta/rosetta <AppImage> serve --bind 127.0.0.1 --port 7005 --trust-folder /tmp/ws1`,
     bound to guest loopback; the script sends it no request. Transcript lines 51 to
     61, at `c5ca91f`, with uncommitted changes (this ticket's notes):

     ```text
     $ method 1 reaching serve: APPIMAGE_EXTRACT_AND_RUN=1 through the interpreter, port 7005, 20 s
     exit 124 (124 = still serving when timeout ended it)
     Trusted /tmp/ws1
     [caps] no usable systemd user slice: systemctl --user --version failed
     [caps] cpu none, memory none (no systemd user manager)
     http://127.0.0.1:7005/?token=<redacted>
     MCP server: http://127.0.0.1:45937/operator
     not signed in — run: quint-studio login
     [loops] read.components failed (missing) after 5.543083ms
     [loops] could not read what the loops landed: sign in required — open Quint Studio and sign in, or run: quint-studio login
     /tmp/appimage_extracted_e12087593f6db4dfc5ccc13ac3f3406c
     ```

     The last line lists the directory just after method 1 made it, so the directory
     outlives the command; it shows nothing about a restart. A restart empties `/tmp`:
     after a `stop` and `start`, `/tmp` in `qs01-s` was empty (claim 8) and a test
     machine's file under `/tmp` was gone (claim 16).
   - `--appimage-extract` into a guest directory: works, and is what the spike used.
     `cd /opt/quint-studio && /mnt/rosetta/rosetta /opt/studio-dl/quint-studio.AppImage --appimage-extract`
     exited 0 in 1 s and left `squashfs-root/`, 790M in 460 files, on the machine's own
     disk. The output of that first extraction was not saved. Two later runs were. One
     is a rerun in `qs01-a` from a mode-0644 copy of the AppImage on the guest's `/tmp`,
     into the empty `/tmp/qs-extract2`, not into the tree at `/opt/quint-studio`.
     Transcript lines 36 to 41, at `c5ca91f`, with uncommitted changes (this ticket's
     notes); after the label come the extraction's status, the number of lines it
     printed, the directory's listing, the count of files and the size in megabytes
     (the script's line 17):

     ```text
     $ method 2 again, into an empty directory, from the non-executable copy
     exit 0
     0
     squashfs-root
     460
     790
     ```

     The other is the provisioning of `qs01-s`, a new machine from the image.
     `ai_tmp/qs01/bin/guest-provision.sh` runs under `set -eu`, checks that
     `squashfs-root` is absent (`test ! -e`), extracts, and only then writes the
     AppImage's SHA-256 to `/opt/quint-studio/extracted-from.sha256`; it printed
     `extracted files: 460` (`logs/s-provision2.txt`). The tree survives a restart. In
     `qs01-a` it was there with its 460 files at 15:18 (transcript lines 42 to 44, which
     the script labels "the tree extracted before the restarts is still there"), after
     the machine had been started again at 15:13 (`logs/a-start-7000.txt`); no saved log
     names the stop before that start. In `qs01-s` the tree and its stamp were there
     after a `stop` and `start`, each exit 0 (claim 8). That `/opt/quint-studio` is on
     the machine's own disk and is not a host mount is read from the script that created
     `qs01-s`, whose twelve mounts are all paths under `/Users/` (`bin/s-create.sh`); the
     guest's `/proc/mounts` was not quoted for it. The entry point is
     `squashfs-root/AppRun`, a bash script generated by linuxdeploy that sources
     `apprun-hooks/linuxdeploy-plugin-gtk.sh` and `exec`s `AppRun.wrapped`; the program
     is `usr/bin/quint-studio`.
   - The host `unsquashfs`: not needed, not tried.

   The change is the guest's libraries: the image alone is not enough. `ldd` on
   `usr/bin/quint-studio`, with the AppDir's `usr/lib` on the path, reported 15 distinct
   libraries `not found`: `libfontconfig.so.1`, `libwayland-client.so.0`, `libX11.so.6`,
   `libfribidi.so.0`, `libfreetype.so.6`, `libxcb.so.1`, `libexpat.so.1`,
   `libharfbuzz.so.0`, `libgbm.so.1`, `libdrm.so.2`, `libgpg-error.so.0`, `libGL.so.1`,
   `libEGL.so.1`, `libX11-xcb.so.1` and `libcom_err.so.2`. Those 15 names come from the
   lead's run in `qs01-a`, whose output was read at the time; no saved log holds them.
   After installing them, every command panicked with
   `No CA certificates were loaded from the system` (from its Sentry client,
   `sentry-0.49.1`), until `ca-certificates` was installed. `serve` then asked for `git`
   (the last line of the first `serve` quote above), which step 8 installs. "Facts for
   QS04" lists every package and version.

   The saved evidence is the state after the installs. In `qs01-a` at 15:18,
   `LD_LIBRARY_PATH=<AppDir>/usr/lib ldd <AppDir>/usr/bin/quint-studio`, with `ldd`'s
   status, the number of lines it printed, the lines holding `=> /`, and
   `grep -c 'not found'` with its status (the script's line 20). Transcript lines 45 to
   50, at `c5ca91f`, with uncommitted changes (this ticket's notes):

   ```text
   $ ldd with the AppDir library path: lines, resolved, not found
   ldd exit 0
   130
   129
   0
   grep 'not found' exit 1 (1 = none)
   ```

   That shows nothing missing in that guest as it stood then. Later steps had run in
   the same machine by 15:18 and may have added packages, and no package listing of
   `qs01-a` was taken then, so it does not show that the 15 and `ca-certificates` are
   enough on a new machine. What `qs01-s` shows is that the whole provisioning set is
   enough: made from the bare image and provisioned by `bin/guest-provision.sh` with
   the 15, `ca-certificates`, `git` and `curl` (`packages set up: 89`,
   `logs/s-provision2.txt`), it served the UI, the sign-in and a scan (claims 5, 6 and
   9). The smaller set, the 15 and `ca-certificates` without `git` and `curl`, was not
   tested on a new machine. `ldd` was not run there. `ldd` reads only the
   binary's declared dependencies: a library opened at run time does not appear, as the
   CA-certificate panic showed. The evidence that `serve` has what it needs is that it
   started and printed its URL, and the requests of the later steps.
5. **Holds with stated changes.** `serve --bind 0.0.0.0 --port 7000 --trust-folder <worktree>`
   was started in the guest by `machine exec -d`, on a machine with `-p 7000:7000` and
   the worktree mounted read-write at its own path. The transcript is from the first
   such machine, `qs01-a` (`--net --rosetta`); the machine with QS04's layout, `qs01-s`
   (`--net --block-private --deny-cidr 127.0.0.0/8 --rosetta`, 12 mounts), gave the same
   `401 Unauthorized` with `content-length: 0` and the same `200` with the token:

   ```text
   $ curl -sS -o /dev/null -w '%{http_code} %{redirect_url}\n' http://127.0.0.1:7000/
   401
   curl exit 0
   $ curl -sS -D - http://127.0.0.1:7000/
   HTTP/1.1 401 Unauthorized
   content-length: 0
   date: Wed, 25 Nov 2026 23:13:39 GMT
   curl exit 0
   $ curl -sS -o /dev/null -w '%{http_code} %{redirect_url}\n' 'http://127.0.0.1:7000/?token=<redacted>'
   200
   curl exit 0
   $ curl -sS -D - -o <file> 'http://127.0.0.1:7000/?token=<redacted>'
   HTTP/1.1 200 OK
   content-type: text/html; charset=utf-8
   set-cookie: quint_studio_token=<redacted>; Path=/; SameSite=Strict
   content-length: 934
   curl exit 0
   $ curl -sS -o /dev/null -w '%{http_code} %{redirect_url}\n' 'http://[::1]:7000/'
   401
   curl exit 0
   ```

   The answer without the token is exactly `401`, with no `Location`, an empty body and
   no `Server` header. That is a refusal as the claim defines one. The body with the
   token is Studio's page (`<title>Quint Studio</title>`).

   *The token* is printed by `serve` on its standard output, in lines of this shape,
   and nowhere else was needed:

   ```text
   http://100.96.0.2:7000/?token=<redacted>
   listening on every interface; both of these work, token included:
     from another device on this network:  http://100.96.0.2:7000/?token=<redacted>
     from this machine:                    http://127.0.0.1:7000/?token=<redacted>
   ```

   It is 64 lowercase hexadecimal characters, the same value in the cookie, and it
   **changes at every `serve` start** (`cmp` of the value before and after a machine
   restart: exit 1). The URL's shape is `http://127.0.0.1:<port>/?token=<64 hex>`.

   *An old token is refused after a restart.* On `qs01-s`, a token read for the test
   answered 200 as the query and as the cookie. After a stop and a start it answered
   401 both ways, and the new token's cookie answered 200.
   `logs/old-token-after-restart.txt` (18:44 local) lines 1 to 20, at `c5ca91f`, with
   uncommitted changes (this ticket's notes). The `$` lines are the log's own account
   of each step, not command lines, and no token value is in the log:

   ```text
   $ machine start --name qs01-s; serve; read token A
   Machine 'qs01-s' running (PID: 98170)
   exec -d serve exit 0
   $ request with token A (query), and with token A as the cookie
   200
   curl exit 0
   200
   curl exit 0
   $ machine stop; machine start; serve; read token B
   Stopped machine: qs01-s
   Machine 'qs01-s' running (PID: 98361)
   exec -d serve exit 0
   cmp token A, token B: exit 1 (1 = differ)
   $ after the restart: token A as query, token A as cookie, token B as cookie
   401
   curl exit 0
   401
   curl exit 0
   200
   curl exit 0
   ```

   That is one stop and start of one machine, with a token read for the test. The
   cookie the maintainer's browser holds was not replayed (section 9).

   The changes:

   - **`-p 127.0.0.1:7000:7000` is not accepted.** `machine create ... -p 127.0.0.1:7000:7000`
     exits 2: `error: invalid value '127.0.0.1:7000:7000' for '--port <PORT[-END]|HOST[-END]:GUEST[-END][/udp]>': invalid host port: 127.0.0.1`.
     The form is `-p 7000:7000`, and smolvm chooses the address: `--help` says "Published
     ports listen on `127.0.0.1` and `[::1]`", and `SMOLVM_PUBLISH_ADDR` in the
     environment widens it (`crates/smolvm-network/src/publish_bind.rs`), so a recipe
     must not inherit that variable.
   - **Port 7000 is already taken on this Mac, by AirPlay Receiver, and smolvm binds
     beside it without an error.** With no spike machine publishing:

     ```text
     $ lsof -nP -iTCP:7000 -sTCP:LISTEN
     ControlCe 641 <user>    9u  IPv4 ...  TCP *:7000 (LISTEN)
     ControlCe 641 <user>   10u  IPv6 ...  TCP *:7000 (LISTEN)
     lsof exit 0
     $ curl -sS -D - -o /dev/null http://127.0.0.1:7000/
     HTTP/1.1 403 Forbidden
     Content-Length: 0
     Server: AirTunes/940.23.1
     ```

     and with the machine up:

     ```text
     $ lsof -nP -iTCP:7000 -sTCP:LISTEN
     ControlCe   641 <user>    9u  IPv4 ...  TCP *:7000 (LISTEN)
     ControlCe   641 <user>   10u  IPv6 ...  TCP *:7000 (LISTEN)
     smolvm-bi 47708 <user>    8u  IPv4 ...  TCP 127.0.0.1:7000 (LISTEN)
     smolvm-bi 47708 <user>    9u  IPv6 ...  TCP [::1]:7000 (LISTEN)
     lsof exit 0
     ```

     Studio is bound on loopback only: the same request to the host's LAN address
     answers `HTTP/1.1 403 Forbidden` with `Server: AirTunes/940.23.1`, which is AirPlay,
     not Studio. So on this host a tokenless `403` on port 7000 means **the machine is
     down**, and it would pass a check that accepts "401 or 403". QS04's Verification
     must require `401` and the absence of the `AirTunes` header, or publish on a host
     port nothing else holds (`-p 7100:7000` booted; `lsof -nP -iTCP:7100 -sTCP:LISTEN`
     exited 1 beforehand).
   - **The host browser was not opened by the agent.** See Departures.

   A file the guest's user creates in the mounted worktree: in the guest, `umask` 0022,
   `-rw-r--r-- 1 0 0` (root). On the host, `ls -ln`: `-rw-------@ 1 501 20`, the
   maintainer's uid and gid, mode 0600, with the extended attributes
   `com.apple.provenance` and `user.containers.override_stat`, where smolvm keeps the
   guest's view of owner and mode. In the guest every worktree file appears owned by
   501:20.
6. **Holds: sign-in works headless, through the forwarded port.** The maintainer signed
   in, in a private browser window opened on the token URL, and reported: "Email. It
   emailed a code to me." That is the UI's counterpart of
   `quint-studio login --email <address>`; `login --help` (exit 0) offers
   "sign in with Google, or a code emailed to --email, and accept the Terms", with
   `--accept-terms` and `--telemetry on|off`. Google sign-in and the CLI `login` were
   not exercised, and the prompts the UI showed were not reported. No device-code flow
   was seen. Afterwards, in the guest:

   ```text
   $ quint-studio status
   ✓ signed in as <redacted>, access expires in 23h 59m
   ✗ no model provider signed in — run: quint-studio providers
   ✓ /Users/scutting/.supacode/repos/pachinko/QS01 is trusted
   ✗ no coding agent connected — run: quint-studio connect <agent>
   ✓ Studio serves /Users/scutting/.supacode/repos/pachinko/QS01 at http://127.0.0.1:7000/
   ✓ usage: month from 2026-11-01 to 2026-12-01: $0.00, no limit
   status exit 0
   ```

   The access it grants expires after a day by that line; what happens then was not
   seen. The maintainer then added two providers in the UI, Anthropic with an API key
   and OpenAI through an OAuth flow, and `status` read
   `✓ model providers signed in: anthropic, openai`. Guest files changed since a marker
   made before sign-in, by step 3's predicate over `/root` and `/tmp` with both
   controls present (`find` wrote nothing to stderr; its exit status was not captured,
   as the next paragraph says), names only, all under
   `/root/.local/share/co.quint.studio/`: `auth.json` and `access.json` at sign-in,
   with `bin/quint-loops` (a 1012-byte executable Studio put there),
   `telemetry/queue.ndjson`, `usage.sqlite-shm`, `usage.sqlite-wal` and
   `projects/BiscuitBin_pachinko/worktrees/<id>`; then `loops-auth.json`,
   `loops-models.json`, `settings.json` and `telemetry/loops-queue.ndjson` when the
   providers were added. No file's content was read. Nothing changed under `/root/.config`,
   which does not exist, and `/root/.cache` holds only `rosetta`.

   Two limits on that listing. `bin/guest-signin-diff.sh` line 5 pipes `find` into
   `sort`, and line 6 prints `$?` under the label `find exit`, so the `find exit 0` of
   `logs/signin-guest-files2.txt` line 22 is `sort`'s status. `find`'s standard error
   went to a file, and that file held no line (`stderr lines: 0`). And the command is
   `find /root /tmp -xdev`, which stays on each root's own filesystem: a filesystem
   mounted beneath `/root` or `/tmp` would not be walked, and both controls are files
   directly in `/root`, so they do not test that. No mount under `/root` or `/tmp` is in
   `bin/s-create.sh`, which made the machine; the guest's `/proc/mounts` was not quoted
   for it. The listing is the paths that were seen, not a proof that no other
   credential-bearing path was written.
7. **Holds with a stated change.** The worktree's `.git` is the one-line file
   `gitdir: /Users/scutting/projects/pachinko/.git/worktrees/QS01`, whose `commondir` is
   `../..`, so the common directory is `/Users/scutting/projects/pachinko/.git` (34M, no
   credential in a remote URL: `grep -cE '://[^/@ ]+@' config` printed 0). With only the
   worktree mounted:

   ```text
   $ git -C <worktree> status
   fatal: not a git repository: /Users/scutting/projects/pachinko/.git/worktrees/QS01
   git status exit 128
   ```

   With the common directory mounted read-only at its host path
   (`-v /Users/scutting/projects/pachinko/.git:/Users/scutting/projects/pachinko/.git:ro`;
   `/proc/mounts`: `virtiofs ro,relatime`), git still refused, which is the change:

   ```text
   fatal: detected dubious ownership in repository at '/Users/scutting/.supacode/repos/pachinko/QS01'
   git status exit 128
   ```

   The guest runs as root and sees the files as 501:20. After
   `git config --global --add safe.directory <worktree>` in the guest (it writes
   `/root/.gitconfig`):

   ```text
   $ git status --short
    M .scratch/quint-studio/QS01-studio-spike.md
   git status exit 0
   $ git log -1 --format='%H %s'
   c5ca91f186dcffd14c7e372529c7e8696195b40f Use Codex Sol as the reviewer in tickets QS01 to QS05
   git log exit 0
   $ git commit --allow-empty -m qs01-probe
   fatal: Unable to create '/Users/scutting/projects/pachinko/.git/worktrees/QS01/index.lock': Read-only file system
   git commit exit 128
   $ touch /Users/scutting/projects/pachinko/.git/qs01-guest-write
   touch: cannot touch '/Users/scutting/projects/pachinko/.git/qs01-guest-write': Read-only file system
   ```

   The error is the guest mount's `Read-only file system`, not the host sandbox's
   `Operation not permitted`, so the read-only mount is what refused it. On the host
   afterwards `git log -1` was still `c5ca91f` and no file had appeared in the common
   directory (`test ! -e`, exit 0).

   *Does Studio run git, and does it write?* It runs git: without it, `serve` printed
   "Studio needs git to open a project, and git does not run on this computer." A
   logging shim in place of `/usr/bin/git` recorded what `serve` ran when it opened the
   worktree, before any sign-in. In the worktree, only these three, all reads:
   `config --get remote.origin.url`, `rev-parse --show-prefix` and
   `rev-parse --show-toplevel`. In its own data directory,
   `~/.local/share/co.quint.studio/projects/BiscuitBin_pachinko/worktrees/<id>/.quint-init-*`,
   it ran `init`, `symbolic-ref HEAD refs/heads/main`, four `config --local` calls
   (`user.name quint-studio`, `user.email quint-studio@localhost`, `commit.gpgsign false`,
   `core.hooksPath /dev/null`) and `commit --allow-empty -m start`: a repository of its
   own, not this one. The project's name there comes from the remote URL. What it runs
   during a signed-in session is recorded under claim 8.
8. **Holds with a stated change: the data survived on the machine's own disk, not on a
   volume.** The mount budget (claim 16) left no slot for a disk volume beside the
   twelve host mounts, so Studio's data directory stayed on the overlay root, which
   upstream's help says persists "across exec sessions and a stop and start". After
   `machine stop --name qs01-s` (exit 0) and `machine start` (exit 0), before `serve`
   was started again:

   ```text
   access.json auth.json bin codebase-memory instances loops-auth.json loops-models.json projects request-limits.sqlite ...
   BiscuitBin_pachinko
   ✓ signed in as <redacted>, access expires in 23h 36m
   ✓ model providers signed in: anthropic, openai
   ✓ /Users/scutting/.supacode/repos/pachinko/QS01 is trusted
   ✗ no Studio has /Users/scutting/.supacode/repos/pachinko/QS01 open — run: quint-studio serve
   ✓ usage: month from 2026-11-01 to 2026-12-01: $0.22, no limit
   status exit 0
   ```

   The extracted tree, its `extracted-from.sha256` stamp, `/root/.gitconfig` and the
   git wrapper were also still there, and `/tmp` was empty. A disk volume's own
   persistence was shown separately with a plain file (claim 16). The data does not
   survive `machine delete`.

   *The project after a restart.* The quote above was taken before `serve` ran again,
   so it shows the project's directory and the sign-in, not Studio answering for the
   project. That was shown after a later start of the same machine, PID 68934 where
   the start above was PID 29963; the last stop before it that a saved log records is
   `logs/reruns-limits.txt` lines 12 and 13 (claim 16). After `machine start --name qs01-s`,
   `serve` was started again, `quint-studio status` was run, and `bin/mcp-call.py`
   called the MCP tool `read_project_summary` three times over the stdio launcher
   (claim 15).
   `logs/project-after-restart.txt` (18:17 local) lines 2 to 13 and 18 to 22, where
   `<W>` is the worktree, at `c5ca91f`, with uncommitted changes (this ticket's notes):

   ```text
   Machine 'qs01-s' running (PID: 68934)
   start exit 0
   $ machine exec -d: serve --bind 0.0.0.0 --port 7000 --trust-folder <W>
   exec -d exit 0
   $ quint-studio status
   ✓ signed in as <redacted>, access expires in 23h 09m
   ✓ model providers signed in: anthropic, openai
   ✓ <W> is trusted
   ✗ no coding agent connected — run: quint-studio connect <agent>
   ✓ Studio serves <W> at http://127.0.0.1:7000/
   ✓ usage: month from 2026-11-01 to 2026-12-01: $0.22, no limit
   status exit 0
   $ python3 mcp-call.py
   read_project_summary call 1: 16.5 ms; isError=False; error=None; answer 7179 characters
   read_project_summary call 2: 6.7 ms; isError=False; error=None; answer 7179 characters
   read_project_summary call 3: 3.2 ms; isError=False; error=None; answer 7179 characters
   exec exit 0
   ```

   Each answer is the project's component map; line 24 of the file holds the start of
   the first, components of this repository with their source paths. So after a stop
   and start Studio is signed in, serves the worktree and answers
   `read_project_summary` with a component map in 3 to 17 ms. No model call was made
   between the scan and a reading of Studio's usage taken after these calls: its
   month total and its line for the model still gave the scan's figures (claim 9).
   `logs/old-token-after-restart.txt` (18:44 local) lines 21 to 26,
   at `c5ca91f`, with uncommitted changes (this ticket's notes); the `$` line is the
   log's own account of the step:

   ```text
   $ quint-studio usage, after three restarts since the scan
   month from 2026-11-01 to 2026-12-01: $0.22, no limit
   by model
     openai/gpt-5.6-luna: 92 calls, in 397347, out 45399, cache read 4214144, cache write 0, $0.22
   by project
   usage exit 0
   ```

   So no scan that calls a model produced the map. That is all the probe shows. That
   the map is byte for byte the one Studio held before the restart was not checked: no
   answer was saved before the stop, and `bin/mcp-call.py` compares nothing with an
   earlier run. A map rebuilt from the mounted repository without a model call would
   give the same lines (Open point 16). This reading also differs in shape from the
   one saved after the scan. There `by project` is followed by a line for this
   worktree, with the same 92 calls, and by the `by loop` section claim 9 quotes
   (`logs/after-scan.txt` lines 12 to 15). Here `by project` is the last line before
   the exit status. Whether the log was cut there or Studio no longer lists the
   project was not looked into, and it bears on the same question. The project list
   in the browser was not looked at after the restart.

   *Every path Studio wrote.* In the guest, everything is under
   `/root/.local/share/co.quint.studio/` (4.6M after the scan): the files claim 6
   lists, plus `codebase-memory/`, `instances/`, `projects/`, `request-limits.sqlite`,
   `usage.sqlite`, `workspaces.json`, `update-check-stable.json` and `telemetry/`; and,
   while it works, `/tmp/loops-index-*/` and `/tmp/loops-workspace-*/QS01`, a copy of
   the worktree it makes for a scan. **In the mounted worktree it wrote nothing**, in
   either view, over a session that held sign-in, both provider logins and a completed
   scan: `git status --short --ignored` before and after was identical (`diff` exit 0),
   and `find . \( -newer <marker> -o -cnewer <marker> \)` with both controls present
   (`find` exit 0, nothing on stderr) listed 36 paths, 35 of them the spike's own under
   `ai_tmp/` and the other `.` itself, from the spike's own probe files at the root.
   The `.git` pointer file's SHA-256 was the same before and after. Studio's git
   commands name a directory it did not create in this session, `quint-specs/`, with a
   `quint-specs/quint.lock`: that is where it would put specifications in the
   worktree, and QS04 should expect it.

   *Studio writes to the repository's object store.* The first scan failed in the UI:

   ```text
   error: unable to create temporary file: Read-only file system
   error: .scratch/quint-studio/QS01-studio-spike.md: failed to insert into database
   error: unable to index file '.scratch/quint-studio/QS01-studio-spike.md'
   fatal: updating files failed
   ```

   A scan snapshots the worktree with a private index, never the real one:
   `GIT_INDEX_FILE=/tmp/loops-index-<x>/<name> git -c core.splitIndex=false --no-optional-locks add -A -- . ':(exclude)quint-specs'`,
   then `write-tree`, then about 1100 `cat-file blob` reads. `add` writes blob objects
   into the object store, which is in the common git directory, mounted read-only
   (claim 7). The same command by hand in the guest gave the same four lines, exit 128.
   The workaround the scan then completed with: the guest's `git` is a wrapper that,
   when `rev-parse --git-common-dir` is under `/Users/`, sets `GIT_OBJECT_DIRECTORY` to
   a directory on the guest's disk, `/root/qs01-git-objects`, and adds the real
   `objects/` to
   `GIT_ALTERNATE_OBJECT_DIRECTORIES`. It does so only when the caller has not set
   `GIT_OBJECT_DIRECTORY` itself, it keeps any alternates already set, behind the real
   store, and it then runs the real binary, moved to `/usr/bin/git.real`, with the
   caller's arguments (`bin/guest-git-shim3.sh` lines 6 to 16). A repository on the
   guest's own disk, outside `/Users/`, is therefore left alone, and Studio keeps one:
   the session's log
   shows git run both in the mounted worktree and in Studio's project under its data
   directory (`logs/git-calls-session2.txt` lines 14 to 23). The tested wrapper also
   logs every call, which is the spike's and no part of the workaround. With it, `add`
   on a private index exits 0 and
   `write-tree` printed a tree id; the same `add` on the real index still fails with
   `Unable to create '.../worktrees/QS01/index.lock': Read-only file system`, which is
   wanted. On the host, `git count-objects` read `5006 objects` before and after. The
   alternative is to mount the common directory read-write, which lets the guest
   rewrite hooks, configuration and every branch.
9. **Holds for the network; sign-in and the provider call are recorded under claim 6.**
   Two host listeners, `python3 -m http.server` on `127.0.0.1:18765` and on the host's
   LAN address (`192.168.x.x`, written `<LAN>`) port 18766, each answering the host
   itself with `qs01-listener-ok`. The control, a machine with `--net` and neither deny
   flag, each line being `curl -sS -m 6 -o /dev/null -w '%{http_code}'` and its exit
   status:

   ```text
   nameserver 100.96.0.1
   gateway 100.96.0.1
   100.96.0.1      host.docker.internal
   gateway 100.96.0.1:18765                                   200 curl exit 0
   host.docker.internal:18765                                 200 curl exit 0
   guest's own 127.0.0.1:18765 (control: not the host)        000 curl exit 7
   <LAN>:18766                                                200 curl exit 0
   192.168.0.1:80 (LAN router)                                200 curl exit 0
   169.254.169.254 (metadata)                                 000 curl exit 7
   https://example.com                                        200 curl exit 0
   https://accounts.google.com                                302 curl exit 0
   https://api.anthropic.com                                  404 curl exit 0
   ```

   So the guest-side address that reaches host loopback is the gateway, `100.96.0.1`,
   also named `host.docker.internal`; the guest's own address is `100.96.0.2`. Then the
   same script in `qs01-s`, created with `--net --block-private --deny-cidr 127.0.0.0/8`:

   ```text
   nameserver 100.96.0.1
   gateway 100.96.0.1
   gateway 100.96.0.1:18765                                   000 curl exit 7 curl: (7) Failed to connect to 100.96.0.1 port 18765 after 5 ms: Could not connect to server
   host.docker.internal:18765                                 000 curl exit 7
   <LAN>:18766                                                000 curl exit 7
   192.168.0.1:80 (LAN router)                                000 curl exit 7
   169.254.169.254 (metadata)                                 000 curl exit 7
   https://example.com                                        200 curl exit 0
   https://studio.quint.sh/install.sh                         200 curl exit 0
   https://accounts.google.com                                302 curl exit 0
   https://api.anthropic.com                                  404 curl exit 0
   2606:4700:10::6814:179a example.com
   getent exit 0
   ```

   Both host listeners still answered the host afterwards (exit 0 each), so the
   failures are the policy and not a dead listener. `machine egress-events --name qs01-s`
   lists each refusal (`connect 100.96.0.1:18765`, `connect <LAN>:18766`,
   `connect 192.168.0.1:80`, `connect 169.254.169.254:80`). The resolver is the gateway,
   `100.96.0.1`, inside the range `--block-private` blocks, and DNS still worked: the
   help says "DNS keeps its own path". `apt-get` over HTTP also worked under the deny
   flags (the machine was provisioned that way).

   *Sign-in and one provider call, on that machine.* Sign-in by emailed code and both
   provider logins happened on `qs01-s`, with the deny flags (claim 6). The maintainer
   then ran a scan from the UI. Studio's own account of it, `quint-studio usage`, exit 0:

   ```text
   month from 2026-11-01 to 2026-12-01: $0.22, no limit
   by model
     openai/gpt-5.6-luna: 92 calls, in 397347, out 45399, cache read 4214144, cache write 0, $0.22
   by loop
     scan: 92 calls, in 397347, out 45399, cache read 4214144, cache write 0, $0.22
   ```

   So 92 calls to one provider succeeded under the deny flags. The Anthropic key was
   registered and not called.
10. **Holds with a stated change: the registration was attempted against a stand-in and
    did not complete. Inspected, never applied on the host.** There is no dry-run mode:
    `connect --help` and `connect claude --help` print the same two lines,
    "connect coding agents to Studio in this repository; with no agent named, show each
    one's connection". It was run in the guest in a throwaway repository,
    `/tmp/qs01-connect/repo`, with `HOME=/tmp/qs01-connect/home`, never in the mounted
    worktree. Three things had to be true for it to act. `OWD` must name the
    repository, because the extracted `AppRun` leaves Studio's working directory at
    `squashfs-root/usr` (without it: "`.../squashfs-root/usr` is not in a git repository",
    exit 1). A `claude` command must be on `PATH`, or it reports
    `Claude Code  Not installed`, and `connect claude` exits 1 with
    `Claude Code MCP configuration failed:` and writes no file (`logs/connect2.txt`).
    And that command must answer. The run recorded here had a stand-in on `PATH` that
    only records its arguments, prints nothing and exits 0 (`bin/guest-fake-claude.sh`).
    Studio then listed `Claude Code  Not connected`, and `connect claude` exited 1. The
    one line of standard error in the saved output, at its end, is
    `read Claude Code plugin response: EOF while parsing a value at line 1 column 0`:
    the stand-in's empty answer to the second call below (`logs/connect3-raw.txt`). So
    no registration completed. The stand-in intercepted the attempt and changed
    nothing. What it was asked to do:

    ```text
    call: cwd=/tmp/qs01-connect/repo argc=8
      arg: mcp
      arg: add
      arg: --scope
      arg: local
      arg: quint-studio
      arg: --
      arg: /opt/quint-studio/squashfs-root/usr/bin/quint-studio
      arg: mcp
    call: cwd=/tmp/qs01-connect/repo argc=4
      arg: plugin
      arg: marketplace
      arg: list
      arg: --json
    ```

    So it tries to register a **stdio** MCP server named `quint-studio`, at local
    scope, whose command is the Studio binary's own path with the argument `mcp`; then
    it asks Claude Code for its plugin marketplaces, and there it stopped. What a real
    `claude` writes for the first call, and what Studio does after a real answer to
    the second, were not seen.

    The files below are what Studio itself had written by the time it failed, not the
    write set of a completed connection. They are the regular files outside `.git/`,
    under the throwaway `HOME` and repository, that the predicate `-newer` or `-cnewer`
    found against a marker made just before `connect claude`
    (`bin/guest-connect.sh` lines 11 to 14). The marker came after three earlier Studio
    commands in the same `HOME`, so what those wrote is not listed: an
    `update-check-stable.json` in Studio's data directory and a `.cache` directory
    were there too (`logs/connect-files.txt` lines 58 to 70). `logs/connect3-raw.txt`
    lines 25 to 45, all 21 paths, with `<H>` for
    `/tmp/qs01-connect/home/.local/share/co.quint.studio/claude-marketplace` and `<R>`
    for `/tmp/qs01-connect/repo/.claude/skills/quint-studio`:

    ```text
    <H>/.quint-studio-sync.json
    <H>/plugins/quint-studio/README.md
    <H>/plugins/quint-studio/hooks/views.tsx
    <H>/plugins/quint-studio/hooks/hooks.json
    <H>/plugins/quint-studio/hooks/register.tsx
    <H>/plugins/quint-studio/hooks/register.test.tsx
    <H>/plugins/quint-studio/hooks/quint.ts
    <H>/plugins/quint-studio/assets/quint-mark.png
    <H>/plugins/quint-studio/.quint-studio-sync.json
    <H>/plugins/quint-studio/tsconfig.json
    <H>/plugins/quint-studio/types/index.d.ts
    <H>/plugins/quint-studio/.claude-plugin/plugin.json
    <H>/plugins/quint-studio/.claude-plugin/types/.gitignore
    <H>/.claude-plugin/marketplace.json
    <R>/.quint-studio-sync.json
    <R>/agents/openai.yaml
    <R>/SKILL.md
    <R>/references/development.md
    <R>/references/troubleshoot.md
    <R>/references/review.md
    <R>/references/setup.md
    ```

    The 7 under `<R>` are 72K and untracked (`git status`: `?? .claude/`), and the
    `.quint-studio-sync.json` among them is
    `{"source": "quint-studio", "version": "f0635214959d6c32"}`. The 14 under `<H>` are
    204K: a Claude Code plugin marketplace named `quint-studio-local`
    (`marketplace.json`) with one plugin, `quint-studio` version `0.2.11` ("Quint Studio
    project and component dashboards for Claude Code.", `plugin.json`), whose
    `hooks/hooks.json` is `{ "modules": ["./register.tsx"] }`. Every file's content is
    in `logs/connect3-raw.txt` from line 63, which is scratch and is not committed.
    These notes carry the names and the small files' content only (Departure 14).

    None of those files holds a credential (the small ones were read whole; the
    TypeScript and Markdown were not searched). The MCP endpoint's URL and transport are
    under claim 15.
11. **Holds with a stated change: the maintainer judged it tolerable, and the latency
    of a UI action was not measured three times.** Measured on `qs01-s`, three times
    each: start-up, an MCP `tools/list`, an MCP tool call that reads the project, and
    the fetch of the UI's page. From `machine start` to the UI answering `401` on host
    loopback, with `serve` started by `machine exec -d` as soon as `start` returned:

    ```text
    run 1: start exit 0 in 581 ms; exec -d exit 0; UI answered 401 (AirTunes header lines: 0) at 1445 ms from start
    run 2: start exit 0 in 447 ms; exec -d exit 0; UI answered 401 (AirTunes header lines: 0) at 1291 ms from start
    run 3: start exit 0 in 440 ms; exec -d exit 0; UI answered 401 (AirTunes header lines: 0) at 1285 ms from start
    ```

    One MCP `tools/list`, over the stdio launcher through `machine exec -i` (claim 15),
    three calls in each of three sessions, timed from the request's write to the
    answer's line:

    ```text
    run 1: spawn to initialize answer 218 ms; tools/list x3: 2.5, 1.4, 1.1 ms; 20 tools; error=None; exec exit 0
    run 2: spawn to initialize answer 187 ms; tools/list x3: 1.6, 0.9, 0.6 ms; 20 tools; error=None; exec exit 0
    run 3: spawn to initialize answer 185 ms; tools/list x3: 2.1, 0.8, 0.9 ms; 20 tools; error=None; exec exit 0
    ```

    One Studio action through that launcher, the MCP tool `read_project_summary`, which
    returns the project's component map: 16.5 ms, 6.7 ms and 3.2 ms in one session,
    each with `isError=False` (`logs/project-after-restart.txt` lines 19 to 21, quoted
    under claim 8). The UI's page, fetched with the token: `200` in 2.6 ms, 0.8 ms and
    0.8 ms (`curl -w '%{time_total}'`, `logs/perf-ui-get.txt`). Neither is a UI action:
    the first is an MCP call and the second fetches the page's HTML.

    The change: **the latency of a UI action in the browser was not measured three
    times**, as step 12 asks, and will not be in this ticket (Departure 13). The one UI
    action that ran was a scan, run once by the maintainer, who gave its duration from
    memory, below. So there is no repeatable baseline for an action in the page, and
    none of the figures above is one. Provisioning a fresh machine (89 packages and the
    extraction) took 30 s.

    The maintainer's verdict, on the scan they ran from the UI, which makes 92 model
    calls: "It ran for about 10m. I wasn't really paying attention. Seems a little long,
    but it's also completely fine for now. Unless there's some sort of obvious speed up
    that can be done, I think that we're good for now." Tolerable. No speed-up was
    looked for; nothing measured above points at the guest as the slow part.
12. **Fails, with a workaround.** `quint-studio --version` is not an option of 0.2.11:

    ```text
    $ /opt/quint-studio/squashfs-root/AppRun --version
    unknown option '--version' (see --help)
    exit 2
    ```

    `-V` and `version` are taken as a workspace to open in a window and panic on
    `Failed to initialize GTK` (exit 101). `--help` (exit 0) lists no version command.
    The version can be read two ways, which agree on `0.2.11`:
    `quint-studio upgrade --check`, which needs the network and printed
    `Quint Studio 0.2.11 is the latest` (exit 0); and the final download URL and its
    `content-disposition`, `Quint%20Studio_0.2.11_amd64.AppImage`. Nothing inside the
    extracted tree names it: the `.desktop` file has no version key, and the binary's
    only `quint-studio/<n>` string is `quint-studio/0.0.0`. For QS03's pin file the string to
    record is `0.2.11`, taken from the final URL at download time, beside the SHA-256;
    `upgrade --check` only says whether the installed build is the newest.
13. **Fails in two of its parts, with a workaround for one. This is the conflict the
    step says to bring to the maintainer.** Tested with plain guest commands on machines
    that had the worktree mounted read-write at its own path, before Studio was given
    that mount.

    *A read-only mount of the same path masks a directory.* For each of `.tools`,
    `.pixi`, `.quint`, `tools/quint/node_modules`, `target`, `fuzz/target`,
    `loadgen/target`, `.claude`, `.codex` and `ai_tmp`, mounted `<path>:<path>:ro` over
    the read-write worktree mount, `/proc/mounts` in the guest shows `virtiofs ro`, the
    guest lists the host's entries (3, 3, 1, 61, 9, 3, 3, 2, 1 and 2), and every write
    fails:

    ```text
    touch: cannot touch '<W>/.tools/qs01-guest-write': Read-only file system        (create exit 1)
    mkdir: cannot create directory '<W>/.tools/qs01-guest-dir': Read-only file system (mkdir exit 1)
    mv: cannot move '<W>/.tools/.crates.toml' to '<W>/.tools/.crates.toml.qs01': Read-only file system (exit 1)
    mv: cannot move '<W>/.tools' to '<W>/.tools.qs01': Device or resource busy       (exit 1)
    ```

    and the same four refusals for each of the other nine. From the host, no
    `qs01-guest-write` or `qs01-guest-dir` existed under any of them afterwards. An
    empty host directory mounted read-only over `.quint` also works: the guest lists 0
    entries, cannot create in it or rename it, and the host's `.quint/` still held
    `apalache-dist-0.62.4`.

    *A read-only mask at the same path stops writes, not reads.* Through such a mask
    the guest sees the real tree: the counts above are the host's entries, and in
    `qs01-s` the guest read the AppImage from under `ai_tmp/qs01/`, through `ai_tmp`'s
    mask, and extracted it (`bin/guest-provision.sh` lines 13 and 14; that machine has
    no other mount of the directory, `logs/rerun-statuses.txt` lines 11 to 22). So in
    the layout that ran Studio the guest could read whatever under `ai_tmp/` the host's
    permissions let smolvm read: the state directory, with the registry pull token
    smolvm cached there (claim 2), and the spike's own token files. Whether the guest
    read any of those was not tested, and nothing in the layout stops it. To hide a
    tree, and not only protect it, the mask must be an empty directory, as shown for
    `.quint`. A state directory needs that kind of mask, not a read-only mount at its
    own path (Open point 14).

    *A single file cannot be masked.* smolvm refuses the mount at create, exit 1:
    `Error: mount operation failed: validate host path: source path on host must be a directory (virtiofs limitation): <W>/qs01-mask-probe`.
    The file that matters is the worktree's own `.git`: in a secondary worktree it is a
    62-byte file inside the read-write mount, and the guest opened it for writing
    (`( : >> <W>/.git )`, exit 0; its SHA-256 on the host was unchanged, since nothing
    was written). A guest that rewrites it chooses the git directory, and so the
    configuration and hooks, that the host's next `git` command in the worktree uses,
    and no diff shows it. In the primary checkout `.git` is a directory and could be
    masked like the others.

    *A writable ancestor of a nested mask can be renamed.* With the nested masks in
    place, from the guest, each `mv <a> <a>.qs01` and then back:

    ```text
    tools/quint: rename exit 0, back exit 0
    tools: rename exit 0, back exit 0
    fuzz: rename exit 0, back exit 0
    loadgen: rename exit 0, back exit 0
    mkdir tools/quint.new exit 0
    ```

    The rename is real on the host. While `loadgen` was renamed, from the host:
    `test -e loadgen` exited 1, `test -d loadgen.qs01/target` exited 0, and
    `git status --short` listed `D loadgen/Cargo.lock` and the rest. So the guest can
    move the masked tree aside and put a tree of its own at the path the host runs.
    Worse, smolvm itself then recreated the mount point: by the next `machine exec`,
    the host had a new, empty, mode-0700 `loadgen/target/`, and the guest's rename back
    landed inside it as `loadgen/loadgen.qs01/` (Departures records the repair).

    *The workaround for that part:* mask each ancestor whole instead of the nested
    path, `tools`, `fuzz` and `loadgen` read-only. Then the ancestors are mount points:
    `mv <W>/tools <W>/tools.qs01` fails with `Device or resource busy`, and
    `tools/quint`, `tools/quint/node_modules`, `fuzz/target` and `loadgen/target` each
    refuse a create and a rename with `Read-only file system`. The cost is that the
    guest cannot edit the tracked files under those three directories. With that
    layout `git status --short --ignored` was identical before and after the guest's
    attempts (`diff` exit 0).

    What this leaves for the maintainer: ruling 2's read-write worktree mount cannot be
    made safe for a secondary worktree by masks alone, because of `.git`; the masks
    that do work consume the machine's mount budget (claim 16); and a mask that is
    read-only at its own path protects a tree without hiding it.
14. **Holds.** Through each registry's HTTP API, each beside its positive control:

    ```text
    https://crates.io/api/v1/crates/smolvm 404
    curl exit 0
    {"errors":[{"detail":"crate `smolvm` does not exist"}]}
    https://crates.io/api/v1/crates/cargo-hack 200
    curl exit 0
    https://api.anaconda.org/package/conda-forge/smolvm 404
    curl exit 0
    {"error":"\"smolvm\" could not be found"}
    https://api.anaconda.org/package/conda-forge/just 200
    curl exit 0
    ```

    Both 404 bodies name absence. See Open points for the PyPI package, which the claim
    does not cover.
15. **Holds with a stated change: the endpoint answered without a credential and no
    option in the two help texts names one, but it is not on port 7001 and cannot be
    published with `-p`.**
    Every `serve` start prints `MCP server: http://127.0.0.1:<port>/operator` with a
    different port (40521, 34395, 39677, 44679 and 45937 were seen). In the guest,
    `/proc/net/tcp` shows two listeners, `0.0.0.0:7000` and `127.0.0.1:<that port>`;
    port 7001 refuses connections (`curl` exit 7), and so does the endpoint's port on
    the guest's own non-loopback address. From the guest, with no token, cookie or
    authorization header:

    ```text
    $ curl -sS -D - -X POST http://127.0.0.1:39677/operator -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"qs01-probe","version":"0"}}}'
    HTTP/1.1 200 OK
    content-type: text/event-stream
    mcp-session-id: <redacted>
    data: {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{"listChanged":true}},"serverInfo":{"name":"rmcp","version":"3.1.4"},"instructions":"Use Quint Studio as a formal-verification backend for the project bound to this MCP session. ...
    ```

    The answer is lines 8, 9, 12 and 20 of `logs/mcp1.txt`; the `$` line is the
    script's command written out, not a line of the log. The log's next line reads
    `curl exit 0`, but `bin/guest-mcp.sh` line 10 pipes `curl` into `head -c 1500` and
    runs an `echo` before it prints `$?`, so that status is the `echo`'s. `curl`'s own
    status was not captured; the evidence is the `HTTP/1.1 200 OK` and the answer. It
    shows that this build, started as claim 5 describes, answered an `initialize` that
    carried no credential. It does not show that no configuration can require one.

    A `GET` on it answers 406. The transport is MCP's streamable HTTP, answering in
    server-sent events. `serve --help` names four options, `--port`, `--bind`,
    `--no-auth` ("serve without the access token") and `--trust-folder`, and one
    variable, `QUINT_STUDIO_NOTIFY_CMD`; `--no-auth` is about the UI's token, and none
    names the MCP server, its port or a credential for it. `connect --help` and
    `connect claude --help` name no option at all.

    *The documentation.* None was found for Studio. After the review the lead fetched
    four URLs with `curl -sS -L`, and the bodies are saved under `ai_tmp/qs01/docs/`
    (18:17 local). `logs/studio-docs-search.txt` (18:40 local) holds each fetch with
    its status, its final URL and `curl`'s exit status, then a search of the saved
    `llms.txt` and a control for it. Lines 1 to 18, at `c5ca91f`, with uncommitted
    changes (this ticket's notes); `<file>` and `<saved llms.txt>` are the log's own
    placeholders:

    ```text
    $ curl -sS -L -o <file> -w "%{http_code} %{url_effective}\n" https://studio.quint.sh/
    200 https://quint.sh/product
    curl exit 0
    $ curl -sS -L -o <file> -w "%{http_code} %{url_effective}\n" https://studio.quint.sh/docs
    404 https://studio.quint.sh/docs
    curl exit 0
    $ curl -sS -L -o <file> -w "%{http_code} %{url_effective}\n" https://studio.quint.sh/llms.txt
    200 https://studio.quint.sh/llms.txt
    curl exit 0
    $ curl -sS -L -o <file> -w "%{http_code} %{url_effective}\n" https://quint-lang.org/docs/studio
    404 https://quint.sh/docs/studio
    curl exit 0
    $ grep -c -i "mcp\|auth\|token\|serve\|headless" <saved llms.txt>
    0
    grep exit 1 (1 = no match)
    $ grep -c -i quint <saved llms.txt>   (positive control)
    32
    grep exit 0
    ```

    `https://studio.quint.sh/` answered 200 at `https://quint.sh/product`, a product
    page whose one mention of MCP is "interact from your agents via local MCP".
    `https://studio.quint.sh/docs` answered 404, and
    `https://quint-lang.org/docs/studio`, saved as
    `https___quint_lang_org_docs_studio.html`, ended at `https://quint.sh/docs/studio`
    with 404. `https://studio.quint.sh/llms.txt` answered 200 with a 57-line index of
    the Quint language's pages. In it the search for the five words counts 0 lines,
    and the control, `quint`, counts 32, so the file was read and the zero is a zero.
    That covers the four URLs tried and no others. So the claim's
    documentation half rests on the two help texts and the observed answer: no page
    was found that could name such an option, which is weaker than a page that says
    there is none.

    What Studio itself wires agents to is not that URL but a stdio launcher (claim 10),
    `quint-studio mcp`: "the stdio MCP launcher an agent CLI is configured with; bridges
    the session to the running Studio whose workspace holds the cwd" (`--help`). It
    works from the host through smolvm, with nothing published:

    ```text
    $ <requests on stdin> | smolvm machine exec -i --name qs01-a -w <worktree> -e OWD=<worktree> -- /opt/quint-studio/squashfs-root/AppRun mcp
    {"id":1,"jsonrpc":"2.0","result":{"capabilities":{"tools":{"listChanged":true}},"instructions":"Use Quint Studio as a formal-verification backend ...
    {"jsonrpc":"2.0","id":2,"result":{"ttlMs":0,"cacheScope":"private","tools":[{"name":"cancel_run", ...
    exec exit 0
    ```

    `initialize` answered with `serverInfo` `rmcp 3.1.4` and `tools/list` with 20 tools
    (`cancel_run`, `close_fixed_bug`, `delete_component_model`, `edit_component`,
    `export_project`, `import_project`, `read_component_entry`,
    `read_component_summary`, `read_pipeline_status`, `read_project_dashboard`,
    `read_project_runs`, `read_project_summary`, `read_questions`, `read_run`,
    `resume_paused`, `run_stage`, `run_state_at`, `run_workflow`, `settle_dispute`,
    `setup_project`), all before any sign-in. Anything that can run that `machine exec`
    on the host, or reach the guest's loopback, can call them.
16. **Holds with stated changes.**

    *Running a command before `serve`.* Three ways, each run at least once.
    `machine exec --name <n> -- <command>` runs a command in a running machine and
    returns its status; it is how every guest command here ran. `machine exec -d`
    "spawn[s] the command in the background and return[s] its PID immediately ... so it
    can host long-lived services" (`machine exec --help`), and is how `serve` was
    started: `exec -d exit 0`, printing the PID. `machine exec -i` keeps stdin open,
    which the MCP launcher needs. The host can also pass a boot command, `machine create
    ... -- <command>` or `machine run ... -- <command>`, the machine's workload; and
    `--init <COMMAND>`, "Run command once, on the machine's first start"
    (`logs/help-machine-create.txt:165-166`), so it is no place for a check that must
    run at every start. `--init` and a workload passed to `create` were read from
    `--help` and not run; `machine run -- <command>` was (`logs/help-machine-run.txt:1-3`,
    and claim 3's quotes). `-w <dir>` and `-e KEY=VALUE` set the working directory and
    environment of an exec (`logs/help-machine-exec.txt`). The machine that ran Studio
    for the maintainer, `qs01-s`, had no boot command and no `--init`:
    `bin/s-create.sh` made it with `machine create` (`logs/s-create.txt`:
    `Created machine: qs01-s`, `Mounts: 12`, `Ports: 1`), and `bin/guest-provision.sh`,
    run in the guest after `machine start`, installed the packages and extracted the
    AppImage (`logs/s-provision2.txt`). The create script printed its exit status to
    the terminal, and no saved log holds it.

    *Volumes have no names.* A volume is either a host directory, `-v HOST:GUEST[:ro]`,
    or "a container path alone (`-v /var/lib/docker`) ... a directory on the machine's
    own disk instead of its overlay root ... it persists across stop and start"
    (`machine create --help`). The second kind belongs to its machine and is deleted
    with it, so a second machine's `-v /data` is its own by construction and nothing is
    shared or named. On a machine made with `-v /data -v /data2`, `/proc/mounts` shows
    `/dev/vda /data ext4` and `/dev/vda /data2 ext4`; after `stop` and `start` a file
    under `/data` and one under `/root` (the overlay root) were both still there (`cat`
    exit 0 each) and one under `/tmp` was gone (exit 1).

    *How each kind is listed.* Host mounts: `machine ls` has a `MOUNTS` column that
    counts them, and `machine ls --verbose` prints each mapping, with the published
    port and the network setting. For `qs01-s`, `logs/rerun-statuses.txt` lines 7 to
    27, at `c5ca91f`, with uncommitted changes (this ticket's notes):

    ```text
    $ smolvm-qs01 machine ls --verbose
    NAME   STATE         CPUS     MEMORY  MOUNTS   PORTS  STORAGE  OVERLAY  BRANCHED FROM
    -------------------------------------------------------------------------------------
    qs01-s stopped          4   8192 MiB      12       1   20 GiB   10 GiB  -
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01 -> /Users/scutting/.supacode/repos/pachinko/QS01
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/.tools -> /Users/scutting/.supacode/repos/pachinko/QS01/.tools (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/.pixi -> /Users/scutting/.supacode/repos/pachinko/QS01/.pixi (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/.quint -> /Users/scutting/.supacode/repos/pachinko/QS01/.quint (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/target -> /Users/scutting/.supacode/repos/pachinko/QS01/target (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/.claude -> /Users/scutting/.supacode/repos/pachinko/QS01/.claude (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/.codex -> /Users/scutting/.supacode/repos/pachinko/QS01/.codex (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp -> /Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/tools -> /Users/scutting/.supacode/repos/pachinko/QS01/tools (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/fuzz -> /Users/scutting/.supacode/repos/pachinko/QS01/fuzz (ro)
      Mount: /Users/scutting/.supacode/repos/pachinko/QS01/loadgen -> /Users/scutting/.supacode/repos/pachinko/QS01/loadgen (ro)
      Mount: /Users/scutting/projects/pachinko/.git -> /Users/scutting/projects/pachinko/.git (ro)
      Port: 7000 -> 7000
      Network: enabled
      Created: 2026-11-25T23:23:25Z

    ls --verbose exit 0
    ```

    The JSON row carries the same count as `mounts` and the mappings as
    `shared_folders`, each with its mode, beside `port_mappings` and `disks`
    (`src/cli/vm_common.rs:3587,3594,3641`). That is read from the source: the one
    saved row (`logs/volume-test.txt` line 10) is cut off after its `image` key.
    Disk volumes, the `-v /data` kind, are in none of these views. `disks` is for host
    disks attached beside the managed ones, and a disk volume is a separate field of
    the record, `disk_volumes` (`src/data/resources.rs:196-199,204-207`), which no line
    that builds either listing reads (read from the source). The saved row agrees as
    far as it goes: it shows `"disks": []` for `qs01-v`, which had two disk volumes.
    What lists them is the guest's `/proc/mounts`, quoted above. `machine status`
    without `--json` names neither kind: by the source it prints the state, memory and
    ports (`src/cli/vm_common.rs:3048-3116`). So QS04 checks host mounts with
    `machine ls --verbose` and a disk volume from inside the guest. The verbose
    listing has a limit of its own, read from the source and not run. The `MOUNTS`
    column counts staged mounts with the others, but the `Mount:` lines come from
    `record.mounts` alone (`src/cli/vm_common.rs:3829,3840-3843`), so a staged mount
    is counted and not printed. For `qs01-s` the two agree, 12 counted and 12 printed.
    `machine status --json` builds the same object as the JSON row (`:3698-3707`), so
    by the source it carries `shared_folders` with each mode, `staged` among them
    (`:3641-3651`). It was not run either. A check that must see every mount should
    compare the count with the lines, or read the JSON once a run has shown its shape.
    The verbose listing prints nothing of the deny rules
    either: `qs01-s` was made with `--block-private --deny-cidr 127.0.0.0/8`
    (`bin/s-create.sh` line 10) and shows only `Network: enabled`.

    *A host file in the state directory.* Written from the host at three depths, then
    `test -f` after each of `start`, `stop` and `delete -f` of that machine:

    ```text
    after start:  state/qs01-host-file exit 0   state/Library/Caches/smolvm/qs01-host-file exit 0   state/Library/Caches/smolvm/vms/<id>/qs01-host-file exit 0
    after stop:   state/qs01-host-file exit 0   state/Library/Caches/smolvm/qs01-host-file exit 0   state/Library/Caches/smolvm/vms/<id>/qs01-host-file exit 0
    after delete: state/qs01-host-file exit 0   state/Library/Caches/smolvm/qs01-host-file exit 0   state/Library/Caches/smolvm/vms/<id>/qs01-host-file exit 1
    ```

    smolvm leaves alone a file at the state root or in its cache directory, and removes
    the machine's own directory, whatever it holds, at deletion.

    *The changes: three limits found on the way, each rerun with its output saved.*

    - **At most 12 host mounts under `--block-private` with `--rosetta`.** The thirteenth
      makes the VMM exit before the agent is ready, with no error naming the cause:

      ```text
      [12 mounts, --block-private --deny-cidr 127.0.0.0/8] exit 0: 24
      [12 mounts, --net] exit 0: 24
      [13 mounts, --block-private --deny-cidr 127.0.0.0/8] exit 1: Error: agent operation failed: start machine: agent operation failed: monitor agent: [2026-11-26T01:50:47Z]: virtio-net: poll loop started guest_ip=100.96.0.2 g
      [13 mounts, --net] exit 0: 26
      [12 host mounts, port] exit 0: 12
      [12 host mounts, disk volume] exit 1: Error: agent operation failed: start machine: ...
      [11 host mounts, disk volume] exit 0: 11
      [11 host mounts, two disk volumes] exit 0: 11
      ```

      A published port costs nothing; any number of disk volumes together cost one
      mount. `--block-private` selects the virtio-net backend ("Both imply `--net` and
      use the virtio-net backend"); plain `--net` takes thirteen. Whether the limit is
      the same without `--rosetta` was not tested. QS04's layout as tested uses all
      twelve: the worktree, ten masks and the common git directory.
    - **The image must be pinned by its `linux/amd64` manifest digest, not the index
      digest.** A named machine under `--block-private` made from the index digest came
      up arm64 even with `--rosetta --oci-platform linux/amd64`, and from the amd64
      manifest digest came up x86_64:

      ```text
      ## named machine from the index digest, under --block-private, with --rosetta --oci-platform linux/amd64
      create exit 0
      Machine 'qs01-n' running (PID: 31350)
      aarch64
      exec exit 0
      ## the same from the amd64 manifest digest
      create exit 0
      Machine 'qs01-n' running (PID: 31387)
      x86_64
      exec exit 0
      ```

      The same happened to a machine created with no network: `qs01-m`, made with
      `--rosetta` and no `--net` from the index digest (`bin/m-create.sh` line 13),
      reported `uname: aarch64` (`logs/m-guest.txt` line 1). An ephemeral
      `machine run --net --rosetta` from the index digest, and a named machine with plain
      `--net`, were x86_64. The cause was not traced by a run. The source offers one,
      unconfirmed: when a named machine's guest cannot pull its image, because it has
      no network or has egress rules, `machine start` fetches the image on the host
      (`src/cli/vm_common.rs:1942-1950`, `src/config.rs:1450-1491`), and that fetch
      passes no platform and resolves an index for the host's architecture
      (`src/image_store.rs:257-262,305-306,484`). An arm64 guest fails late and
      confusingly: packages install, the AppImage extracts through the interpreter, and
      only Studio's binary does not run.
    - **A second machine in the same state root cannot be stopped or deleted while the
      Studio machine runs.**

      ```text
      $ machine stop --name qs01-n
      Error: agent operation failed: admit VM startup: an executing startup was not included in the environment review
      stop qs01-n exit 1
      $ machine delete --name qs01-n -f
      Error: agent operation failed: admit VM startup: an executing startup was not included in the environment review
      delete qs01-n exit 1
      ```

      After `machine stop --name qs01-s`, both exited 0. It happened three times, each
      time with the running machine mounting the whole worktree, state directory
      included; earlier, a second machine stopped normally while the running one mounted
      only the AppImage's directory. The message comes from
      `src/db/launch_admission.rs`, which compares each live launch's paths with the
      paths being released. That the worktree mount is what overlaps is an inference
      from those two observations and was not tested further. One machine per state
      root avoids it.

### 5. Facts for QS02

- **Release asset.** `smolvm-1.37.4-darwin-arm64.tar.gz`, from
  `https://github.com/smol-machines/smolvm/releases/download/v1.37.4/smolvm-1.37.4-darwin-arm64.tar.gz`,
  which redirects to `release-assets.githubusercontent.com` under a signed, short-lived
  URL. Published and computed SHA-256:
  `631be08a4e2cd557a1d47963b2d134e7d65a4b15966751730f0553df16f515e0`. The checksum file is
  `https://github.com/smol-machines/smolvm/releases/download/v1.37.4/checksums.sha256`.
- **The tree.** 938 entries, all under one top-level `smolvm-1.37.4-darwin-arm64/`, none
  absolute and none holding `..`. It unpacks to 157M and 425 files:

  ```text
  smolvm                     2.3 KB  bash launcher
  smolvm-bin                 38 MB   Mach-O 64-bit executable arm64
  lib/libkrun.dylib          7.8 MB
  lib/libkrunfw.5.dylib      14 MB   (lib/libkrunfw.dylib is a symlink to it)
  lib/libepoxy.0.dylib       1.2 MB
  lib/libMoltenVK.dylib      4.9 MB
  lib/libvirglrenderer.1.dylib 2.2 MB
  agent-rootfs/              the guest agent's root filesystem, an Alpine tree
  overlay-default.ext4.zst, overlay-template.ext4.zst, storage-template.ext4.zst
  checksums.txt, README.txt
  ```

  The tarball's own `checksums.txt` lists `smolvm`, `smolvm-bin` and the six `lib/`
  entries, and `shasum -a 256 -c checksums.txt` passes all eight, exit 0. It does not
  cover `agent-rootfs/` or the three `.zst` templates.
- **The launcher, not the binary, is what runs.** `README.txt`: "Run the 'smolvm' script
  (not smolvm-bin directly)". The script is `#!/usr/bin/env bash`. It resolves its own
  directory, exports `SMOLVM_AGENT_ROOTFS` to the bundled `agent-rootfs/` unless the
  variable is already set, prepends `lib/` to `DYLD_LIBRARY_PATH`, and `exec`s
  `smolvm-bin`. An inherited `SMOLVM_AGENT_ROOTFS` or `DYLD_LIBRARY_PATH` therefore
  reaches the binary, as `JAVA_OPTS` reached Apalache's launcher in AP02.
- **Entitlements.** `com.apple.security.hypervisor`,
  `com.apple.security.cs.disable-library-validation` and
  `com.apple.security.cs.allow-jit`, the same three as upstream's `smolvm.entitlements`
  at the tag. The signature is ad hoc (claim 1).
- **The version line**, whole, exit 0:

  ```text
  $ smolvm --version
  smolvm 1.37.4
  ```

- **No registry package** on conda-forge or crates.io (claim 14).

- **State and the redirect** (claim 2). Default state, from upstream's
  `docs/teardown/references/locations.md` (observed there on v1.14.2 to v1.23.0) and the
  source at the tag, never observed by the spike, which did not run without the
  redirect: `~/Library/Application Support/smolvm`, `~/Library/Caches/smolvm` and its
  sibling caches, and `~/.config/smolvm`. With `SMOLVM_DATA_DIR` set, smolvm makes that
  directory `HOME` and clears the `XDG_*` roots (`src/process.rs:1575-1576`,
  `:1618-1621`), so every path derived from them moves. The spike saw the database, the
  usage journal, the image seeds, the registry tokens and the per-machine directories
  there (claim 2), and a volume is a directory on its machine's own disk (claim 16).
  The first probe saw no `.config/smolvm`, so that one rests on the source. Three
  writes do not follow the variable: the two templates beside the binary, libkrun's
  `$TMPDIR/krun-eventfd-*`, and `/tmp/ks-*` when a socket path is too long. For this
  repository the value must be a **relative** path, set in the environment, with the
  working directory at the worktree root: an absolute root inside a worktree makes
  socket paths longer than 103 bytes, and smolvm then writes `/tmp/ks-*`. The rule is
  the length of the socket path as it is written, not whether it is relative: root, 44
  bytes and the socket's name must together be at most 103 bytes
  (`libkrun src/socket/src/lib.rs:35,42`), so the root as written is at most 42 bytes
  for `port-forward.sock`. `ai_tmp/qs01/state` is 17. A longer relative root falls back
  to `/tmp/ks-*` (`:61`) exactly as an absolute one does. `--state-dir` always produces
  an absolute root, so it cannot be used. Every later command, `ls`, `exec`, `stop`,
  `delete`, needs the same variable and the same working directory. Without the
  variable it reads the user's default state. With the variable and another working
  directory it does neither: the relative value is used as given
  (`src/process.rs:1575-1576`), `create_dir_all` makes it under that directory
  (`:1596`), and the command sees a second, empty state tree there. The second case is
  read from the source, not run. The spike's wrapper and its two sandbox profiles
  (`ai_tmp/qs01/bin/`) name this worktree's absolute path and are scratch; QS02 writes
  its own.
- **What a recipe must also set or clear.** `DOCKER_CONFIG`, to a directory with no
  `config.json`, or smolvm points it at the real `~/.docker`. `TMPDIR`, into the
  worktree, for libkrun's `krun-eventfd-*`. No inherited `SMOLVM_*` variable
  (`SMOLVM_USAGE_DIR`, `SMOLVM_CONFIG`, `SMOLVM_AGENT_ROOTFS`, `SMOLVM_MACHINE_NAME` are
  each read), `XDG_*`, `DYLD_LIBRARY_PATH` or proxy variable. The spike ran every command
  through `env -i` with `PATH`, `HOME`, `TMPDIR`, `TERM`, `DOCKER_CONFIG` and
  `SMOLVM_DATA_DIR` only, and that was enough for every command quoted here.
- **smolvm writes into its own install directory**: `overlay-default.ext4` and
  `storage-template.ext4`, on first use (claim 2).
- **It keeps a usage journal**, `Library/Application Support/smolvm/usage/events.jsonl`
  and `.../usage/install_id` under the state root (`src/usage.rs:144-158`:
  `<data_local_dir>/smolvm/usage` unless `SMOLVM_USAGE_DIR` is set), one JSON line per
  create, start and delete. Upstream's `src/usage.rs` says
  recording is on by default, `SMOLVM_USAGE=0` turns it off, and nothing is uploaded
  unless `[usage] share = true` is set. The spike left it at the default.

- **Flags used**, each on `machine create` or `machine run`, each seen to work:
  `--image <ref>`; `--rosetta`; `--net`, which `machine run`'s image pull needs (Facts
  for QS04, Image); `--block-private`; `--deny-cidr 127.0.0.0/8`;
  `-p HOST:GUEST`, with no address (claim 5); `-v HOST:GUEST` and `-v HOST:GUEST:ro` for
  host directories, never a file (claim 13); `-v GUEST` for a directory on the machine's
  disk (claim 16); `--name`. `--oci-platform linux/amd64` was accepted and did not select
  the platform (claim 16). On `machine update`, for a stopped machine: `-v`, `-p` and
  `--remove-port`. `machine delete` needs `-f` to skip its prompt.
- **A machine's network policy is not fixed at creation.** `--block-private` and
  `--deny-cidr` exist on `run` and `create` only, and `machine update --help` names
  neither. But `machine update` has `--net` and `--no-net`
  (`logs/help-machine-update.txt:50-54`). In the source, turning networking off clears
  the deny rules and the private-range block with the rest of the egress policy
  (`src/cli/machine.rs:7292-7294,7304-7306`; a deny CIDR is kept as an egress rule,
  `:5697-5700`), and turning it on again sets only `network = true` (`:7957-7960`) and
  restores neither. A machine taken through `--no-net` and then `--net` would run with
  networking on and no deny. This is read from the help and the source, not run. QS04
  never toggles networking on an existing machine. `machine ls --verbose` does not
  print the deny rules (claim 16), so the check that they hold is a probe from the
  guest, as in claim 9.
- **A recipe must use one profile of environment and directory for every command.**
  See "State and the redirect" above; and `SMOLVM_PUBLISH_ADDR` widens the address
  published ports bind (claim 5).
- **A Rosetta preflight** should test for
  `/Library/Apple/usr/libexec/oah/RosettaLinux/rosetta` and refuse before it creates a
  machine: smolvm itself does not refuse (claim 3).
- **Installing a new release over an old one.** An expanded template is never
  refreshed. smolvm looks for the plain `.ext4` in every root before it looks for any
  `.zst`, and returns it as it is (`crates/smolvm-pack/src/assets.rs:578-605`,
  `resolve_template`); when it does expand an archive it returns an existing
  destination without comparing the two (`:632-640`, `expand_template`). A new release
  unpacked over an old install therefore keeps the old `overlay-default.ext4` and
  `storage-template.ext4`, and no checksum sees it: `checksums.txt` does not cover the
  templates. QS02 installs each version into a new, empty directory, or removes both
  `.ext4` files when the pin changes. Neither is the whole rule. The roots searched
  also include `<home>/.smolvm` (`:566-568`), ahead of the binary's directory, and
  when the directory beside the archive cannot be written the expanded file is made
  in, and later reused from, `smolvm/` under the cache directory (`:632-640`). Under
  the redirect that home is the state root. So a state root kept across an upgrade
  can supply an old template whatever the install directory holds, and QS02's rule
  has to cover the state root's `.smolvm/` and cache as well as the binary's
  directory. Read from the source; the upgrade case was not run.
- **A stale alias path.** If a recipe ever passes an absolute state root, smolvm creates
  `/tmp/ks-*`; the denial log in section 9 is how the spike saw it.

### 6. Facts for QS03

- **The `stable/latest` URL**, from `https://studio.quint.sh/install.sh` (HTTP 200, read
  and never run; `HOST=https://studio.quint.sh`,
  `url="$HOST/dl/$CHANNEL/latest/$platform"`, `platform=appimage-x86_64` for Linux
  `x86_64 | amd64`, and no other Linux architecture):
  `https://studio.quint.sh/dl/stable/latest/appimage-x86_64`.
- **Its redirects.** `curl -fsSIL` cannot follow them: the first hop answers 307 and the
  second answers `HTTP/2 405` to a `HEAD`, so the command exits 56. A `GET` with the
  headers saved (`curl -fsSL -D headers.txt -o <file> -w '%{http_code} %{num_redirects} %{url_effective}\n'`,
  exit 0) shows two redirects and a 200:

  ```text
  HTTP/2 307
  location: https://quint-studio-delivery.quint-studio.workers.dev/dl/stable/latest/appimage-x86_64
  HTTP/2 302
  location: https://quint-studio-delivery.quint-studio.workers.dev/dl/stable/0.2.11/linux-x86_64-appimage/Quint%20Studio_0.2.11_amd64.AppImage
  HTTP/2 200
  content-type: application/vnd.appimage
  content-length: 202680824
  content-disposition: attachment; filename*=UTF-8''Quint%20Studio_0.2.11_amd64.AppImage
  etag: "e12087593f6db4dfc5ccc13ac3f3406c"
  200 2 https://quint-studio-delivery.quint-studio.workers.dev/dl/stable/0.2.11/linux-x86_64-appimage/Quint%20Studio_0.2.11_amd64.AppImage
  ```

- **The final URL carries a version, which the Context's upstream reading did not
  expect.** Fetched directly, with no redirect, it answered 200 and the same 202680824
  bytes (`curl -sS -o /dev/null -w '%{http_code} %{num_redirects} %{size_download}\n'`:
  `200 0 202680824`, exit 0). A guess at a versioned form of the first URL,
  `https://studio.quint.sh/dl/stable/0.2.11/appimage-x86_64`, answered 404. No response
  header and no line of `install.sh` names a checksum.
- **SHA-256 on the day**, 202680824 bytes. At `c5ca91f`, with uncommitted changes (this
  ticket's notes):

  ```text
  $ shasum -a 256 ai_tmp/qs01/studio-download/quint-studio.AppImage
  04a820c83b6321a535bb3923de49fd4c269cf9ba6876d79dedadeb13b907532c  ai_tmp/qs01/studio-download/quint-studio.AppImage
  shasum exit 0
  ```

- **The file** is `ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), static-pie
  linked` (`file`), so its AppImage runtime needs no C library from the guest.

- **No `--version`** (claim 12). The version string for the pin file is `0.2.11`, read
  from the final URL.
- **Extraction** (claim 4): in the guest,
  `/mnt/rosetta/rosetta <AppImage> --appimage-extract`, run from the directory that is
  to hold `squashfs-root/`. The AppImage cannot be executed directly under Rosetta
  (claim 3), and the host cannot run it at all. Through the interpreter the AppImage
  needs to be readable, not executable: a mode-0644 copy on the guest's `/tmp` ran
  `--appimage-version` and `--appimage-extract`, exit 0 each
  (`logs/transcripts-s45.txt` lines 19 to 21 and 36 to 41, quoted under claims 3 and
  4).
  The file on the read-only mount was mode 0755 when that transcript ran (lines 7 to
  9). The saved logs put its change of mode on the host between the fifth and sixth
  boots (`logs/boot5-rosetta.txt:5`, `Permission denied`; `logs/boot6--rosetta.txt:5`,
  `Exec format error`), before any interpreter run, so a 0644 file on the read-only
  mount itself was never run through the interpreter. Upstream's installer sets the
  execute bit (`ai_tmp/qs01/docs/studio-install.sh:100`, `chmod +x`). The spike mounted
  the download directory read-only and worked from that file; the copies above were
  made only for this test and removed.
- **A stale extraction is possible.** The extracted tree lives on the machine's disk and
  nothing in Studio ties it to the AppImage it came from, so a later boot would run the
  old tree after the AppImage changed. The machine the spike tested has a stamp,
  `/opt/quint-studio/extracted-from.sha256`, holding the SHA-256 of the AppImage,
  written by `ai_tmp/qs01/bin/guest-provision.sh` after an extraction into a directory
  it had checked to be absent (claim 4). That stamp does not prove which bytes were
  extracted: the script extracts from the mounted path and then opens the same path
  again to hash it (lines 13 and 14), and the mount is read-only for the guest only,
  so the host can replace the file between the two. The tree would then hold one
  AppImage and the stamp name another, and a check against the new pin would pass.
  Extraction and stamp must come from one copy that cannot change between them: for
  example a copy on the guest's disk that is hashed against the pin first and
  extracted only then. The check that reads the stamp is not built: QS03 or
  QS04 needs that stamp beside `squashfs-root/`, checked before every `serve`. The rule
  fails closed: a missing stamp, a stamp that differs from the pinned AppImage's digest,
  or a re-extraction that does not exit 0 each stops `serve`; none falls through to the
  tree already there. A re-extraction goes into a new, empty directory, and the tree
  and its stamp are put in place together only after it succeeds, so a half-replaced
  `AppRun` is never at the fixed path. The check cannot live in the Smolfile's `init`
  or `machine create --init`. The first is "run once as root, on the machine's first
  start" (`docs/smolfile.md:60`) and the second is "Run command once, on the machine's
  first start" (`logs/help-machine-create.txt:165-166`), so an existing machine started
  after the pin changed would skip it. It belongs in the command that starts `serve`,
  every time.
- **Studio can replace itself.** `--help` lists `quint-studio upgrade`, "install the
  newest release". After `upgrade --check` and `status` had run, the guest's
  `~/.local/share/co.quint.studio/` held `update-check-stable.json`. A pin by hash has to
  account for that command and for any update the running UI offers.

### 7. Facts for QS04

- **Image.**
  `docker.io/library/debian@sha256:ab3b3c8c5159eebfca4376430132d53c4d425f869c765ecbe47177a030da575d`,
  the `linux/amd64` manifest of `debian:trixie-slim` on 2026-11-25 (Debian 13.8), whose
  index digest is
  `sha256:8aa5a7bf5a1955bddbe7d74fcdd0b91ab30e74362a8b73af672b0481b35b44a0`. Pin the
  manifest digest: the index digest gave an arm64 guest under `--block-private`
  (claim 16). One ephemeral `machine run` with networking off, the default, failed in
  a pull its guest made: `crane manifest failed`, ending
  `connect: network is unreachable`, then smolvm's own
  `Hint: networking is disabled. Add --net to enable image pulls`
  (`logs/boot2.txt:3,9-10`; the log holds no command line and no exit line, so neither
  is quoted; `--net` turns networking on, and `--block-private` implies it). That is
  all the run shows. It is not every ephemeral pull, and not every first pull. By the
  source, an ephemeral run that carries a host list, allow CIDRs or egress rules has
  its image fetched on the host before the guest starts, unless the host list names
  every registry host, and a deny CIDR is kept as an egress rule
  (`src/cli/machine.rs:1222-1231,1727,2178-2204`,
  `src/cli/vm_common.rs:2400-2408`). That fetch passes the platform
  `--oci-platform` or `--rosetta` asks for (`:2186-2189`). It is read from the source
  and was not run. So a run with `--deny-cidr` is not shown to pull through the
  guest's egress policy: the registry traffic may be the host's. For a named machine
  whose guest cannot pull, the source has `machine start` fetch the image on the host
  (`src/cli/vm_common.rs:1942-1950`, `src/config.rs:1450-1491`); that is read from the
  source and was not run as a first pull. One named machine with no network did run,
  `qs01-m` (claim 16). Its start log shows no pull (`logs/m-start3.txt`), and whether
  its image came from the earlier boots' cache or from a fetch on the host was not
  recorded. So `--net` is not shown to be needed only to bring the image to a named
  machine. By the source, `machine start`'s fetch on the host of an index reference
  is for the host's architecture (claim 16), another reason to pin the manifest
  digest. The
  Smolfile keys upstream documents are `image = "..."`, `net = true`, default false
  (`docs/smolfile.md:38,51`), and `rosetta` (named among "the other accepted keys" at
  `:113-117`, with no example).
- **Guest packages.** The image cannot carry them: it is Debian's own. They were
  installed in the running machine with
  `apt-get install -y -q --no-install-recommends`, after `apt-get update`, from
  `deb.debian.org` as the image's sources list has it, which is unpinned: the same
  command on another day installs other versions, and an exact version asked for later
  may be gone from the mirror. A later ticket needs either an image of its own, built
  once and pinned by digest, or `snapshot.debian.org` at a recorded timestamp; neither
  was tried. What a snapshot needs is recorded only in part. In the first machine,
  `qs01-a`, the install of the 15 fetched 56 packages, all from
  `http://deb.debian.org/debian trixie/main` (`logs/apt-install.log`: 56 `Get:` lines,
  every one `trixie/main`, none from `trixie-security` or `trixie-updates`), on
  2026-11-25 at about 22:51 UTC, after an `apt-get update` that also read
  `trixie-security` (`logs/apt1.txt:1`). The suite each of the 89 below came from in
  `qs01-s` was not recorded: that machine's install log stayed in the guest and was
  gone when it was asked for (`logs/s-packages.txt`). `package=version` against the
  live mirror is not a pin: a superseded version leaves the live suite. The spike
  install-tested neither a snapshot timestamp nor a derived image; that is QS04's, and
  is listed under Open points. The 18 named on the command line (`apt-mark showmanual`
  in `qs01-s`):
  the 15 for the missing libraries, `libfontconfig1`, `libwayland-client0`, `libx11-6`,
  `libfribidi0`, `libfreetype6`, `libxcb1`, `libexpat1`, `libharfbuzz0b`, `libgbm1`,
  `libdrm2`, `libgpg-error0`, `libgl1`, `libegl1`, `libx11-xcb1` and `libcom-err2`;
  `ca-certificates`; `git`; and `curl`, which only the spike's own probes used. With
  their dependencies that is 89 packages on top of the image's 78 (167 in all, 149
  `amd64` and 18 `all`), by `dpkg-query -W` in `qs01-s` against the same query in a bare
  machine from the image:

  ```text
  ca-certificates 20250419+deb13u1, curl 8.14.1-2+deb13u7, fontconfig-config
  2.15.0-2.3, fonts-dejavu-core 2.37-8, fonts-dejavu-mono 2.37-8, git
  1:2.47.3-0+deb13u1, git-man 1:2.47.3-0+deb13u1, libatomic1 14.2.0-19, libbrotli1
  1.1.0-2+deb13u1, libcom-err2 1.47.2-3+b12, libcurl3t64-gnutls 8.14.1-2+deb13u7,
  libcurl4t64 8.14.1-2+deb13u7, libdrm-amdgpu1 2.4.124-2, libdrm-common 2.4.124-2,
  libdrm-intel1 2.4.124-2, libdrm2 2.4.124-2, libedit2 3.1-20250104-1, libegl-mesa0
  25.0.7-2+deb13u1, libegl1 1.7.0-1+b2, libelf1t64 0.192-4, liberror-perl 0.17030-1,
  libexpat1 2.9.0-1~deb13u1, libffi8 3.4.8-2, libfontconfig1 2.15.0-2.3, libfreetype6
  2.13.3+dfsg-1+deb13u2, libfribidi0 1.0.16-1, libgbm1 25.0.7-2+deb13u1,
  libgdbm-compat4t64 1.24-2, libgdbm6t64 1.24-2, libgl1 1.7.0-1+b2, libgl1-mesa-dri
  25.0.7-2+deb13u1, libglib2.0-0t64 2.84.4-3~deb13u5, libglvnd0 1.7.0-1+b2,
  libglx-mesa0 25.0.7-2+deb13u1, libglx0 1.7.0-1+b2, libgnutls30t64 3.8.9-3+deb13u4,
  libgpg-error0 1.51-4, libgraphite2-3 1.3.14-2+deb13u1, libgssapi-krb5-2
  1.21.3-5+deb13u2, libharfbuzz0b 10.2.0-1+deb13u1, libidn2-0 2.3.8-2, libk5crypto3
  1.21.3-5+deb13u2, libkeyutils1 1.6.3-6, libkrb5-3 1.21.3-5+deb13u2, libkrb5support0
  1.21.3-5+deb13u2, libldap2 2.6.10+dfsg-1, libllvm19 1:19.1.7-3+b1, libnghttp2-14
  1.64.0-1.1+deb13u2, libnghttp3-9 1.8.0-1, libngtcp2-16 1.11.0-1+deb13u1,
  libngtcp2-crypto-gnutls8 1.11.0-1+deb13u1, libp11-kit0 0.25.5-3, libpciaccess0
  0.17-3+b3, libperl5.40 5.40.1-6+deb13u1, libpng16-16t64 1.6.48-1+deb13u6,
  libpsl5t64 0.21.2-1.1+b1, librtmp1 2.4+20151223.gitfa8646d.1-2+b5, libsasl2-2
  2.1.28+dfsg1-9, libsasl2-modules-db 2.1.28+dfsg1-9, libsensors-config 1:3.6.2-2,
  libsensors5 1:3.6.2-2, libssh2-1t64 1.11.1-1+deb13u2, libtasn1-6 4.20.0-2+deb13u1,
  libunistring5 1.3-2, libvulkan1 1.4.309.0-1, libwayland-client0 1.23.1-3,
  libwayland-server0 1.23.1-3, libx11-6 2:1.8.12-1, libx11-data 2:1.8.12-1,
  libx11-xcb1 2:1.8.12-1, libxau6 1:1.0.11-1, libxcb-dri3-0 1.17.0-2+b1, libxcb-glx0
  1.17.0-2+b1, libxcb-present0 1.17.0-2+b1, libxcb-randr0 1.17.0-2+b1, libxcb-shm0
  1.17.0-2+b1, libxcb-sync1 1.17.0-2+b1, libxcb-xfixes0 1.17.0-2+b1, libxcb1
  1.17.0-2+b1, libxdmcp6 1:1.1.5-1, libxext6 2:1.3.4-1+b3, libxml2
  2.12.7+dfsg+really2.9.14-2.1+deb13u3, libxshmfence1 1.3.3-1, libxxf86vm1
  1:1.1.4-1+b4, libz3-4 4.13.3-1, mesa-libgallium 25.0.7-2+deb13u1, openssl
  3.5.9-1~deb13u1, perl 5.40.1-6+deb13u1, perl-modules-5.40 5.40.1-6+deb13u1
  ```

  In `qs01-a`, after the installs, `ldd` on `usr/bin/quint-studio` with the AppDir's
  `usr/lib` on the library path printed 130 lines, 129 of them resolved to a path and
  none `not found`. Claim 4 quotes it and says what it does not show: `ldd` was not run
  in `qs01-s`, and it does not see a library opened at run time.
- **Running Studio.** `OWD=<worktree> /opt/quint-studio/squashfs-root/AppRun <command>`.
  Without `OWD`, cwd-based commands (`connect`, `status`, `mcp`, and `serve` with no
  workspace) see `squashfs-root/usr` as the working directory. `serve` takes
  `--bind 0.0.0.0 --port <n> --trust-folder <worktree>`; `--trust-folder` answers the
  "Trust this folder?" question that would otherwise wait for a person.
- **Studio's data paths.** `/root/.local/share/co.quint.studio/` in the guest, for
  everything including sign-in (claims 6 and 8). On the machine's overlay disk it
  survives `stop` and `start` and is lost at `delete`.
- **Paths Studio writes in the worktree**, in both of step 9's views: none in the
  session recorded under claim 8; `quint-specs/` is where its git commands expect its
  specifications.
- **The git mount that works.** The common git directory read-only at its host path,
  `git config --global --add safe.directory <worktree>` in the guest, and a `git`
  wrapper that sends new objects to the guest's disk (claims 7 and 8). The tested
  wrapper, `bin/guest-git-shim3.sh`, redirects when `GIT_OBJECT_DIRECTORY` is unset or
  empty and the common directory, as `git rev-parse --git-common-dir` prints it,
  starts with `/Users/`. New objects then go to `/root/qs01-git-objects`. It keeps any
  alternates already set and passes every call to the real `git` (claim 8). It does
  not compare the path with the mount: a second repository whose common directory
  prints as a path under `/Users/` would be redirected into the same directory, and a
  path printed in any other form would not be redirected at all. QS04's wrapper should
  compare with the exact mounted path. One that redirects without those conditions
  would override a caller's own object directory and would also divert the objects of
  the repositories Studio keeps on the guest's disk. `git` must be installed in the
  guest.
- **Masks** (claim 13). A directory can be masked read-only at its own path, or by an
  empty directory. A single file cannot. A nested mask's ancestors must be masked
  whole. The layout that ran Studio: the worktree read-write, and `.tools`, `.pixi`,
  `.quint`, `target`, `.claude`, `.codex`, `ai_tmp`, `tools`, `fuzz` and `loadgen`
  read-only, plus the common git directory: twelve mounts, the limit (claim 16). The
  state directory was inside `ai_tmp` and so under its mask; a state directory
  elsewhere needs a mount of its own, and there is none to spare. That mask kept the
  guest from writing the state directory and hid none of it. A read-only mount of a
  path over itself stops writes, not reads, so the guest could read the state tree,
  smolvm's cached registry pull token included, and the rest of `ai_tmp/` (claim 13).
  To hide a tree, mask it with an empty directory, as shown for `.quint`. A state
  directory should be masked that way and not read-only at its own path. In the
  tested layout that is a thirteenth mount, and an empty mask inside a read-only one
  was not tested (Open point 14). For the `.git`
  pointer file, `chflags uchg` on the host made the guest's append, truncation, rename,
  replacement by rename and `chmod` all fail (`I/O error` or `Operation not permitted`),
  with the file's SHA-256 unchanged; it was tried on a throwaway file at the worktree
  root, never on `.git`, and removal by the guest was not tried. The Smolfile key for
  mounts is `volumes = ["HOST:GUEST"]`, with `$VAR` expanded in host paths
  (`docs/smolfile.md:58`). Two traps in that, read from the source and not run, since
  the spike passed explicit paths as flags (`bin/s-create.sh`). The expansion reads
  the process environment after smolvm has applied the state root, which replaces
  `HOME` (`src/main.rs:235-238`, `src/process.rs:1618`,
  `src/cli/smolfile.rs:134-142,334-337`). So under the redirect `$HOME` in a Smolfile
  is the state root, not the user's home; and only the host half is expanded, so a
  variable in the guest half stays as written. And a host path that is missing, or is
  not a directory, fails the create (`src/data/storage.rs:188-202`; claim 13 saw the
  second case), so a mask list copied into a fresh worktree that has no `target` or
  `.pixi` yet stops there.
- **The guest-side address that reaches host loopback** in the control boot:
  `100.96.0.1`, the gateway, also `host.docker.internal` (claim 9). Smolfile keys:
  `[network] block_private = true` and `deny_cidrs = ["127.0.0.0/8"]`.
- **The token and the answer without it** (claim 5): read from `serve`'s standard
  output, 64 hex characters, new at every start; `401` with an empty body and no
  `Server` header. Port 7000 is AirPlay's on this host. Smolfile key:
  `ports = ["HOST:GUEST"]`.
- **Running a command before `serve`, volumes, and a host file in the state
  directory**: claim 16. Smolfile keys: `init = ["..."]`, first start only
  (`docs/smolfile.md:60`); `entrypoint` and `cmd` for the workload (`:39-40`);
  `volumes = ["/guest/path"]` for a directory on the machine's disk. The spike ran none
  of those keys: it used flags and `exec`. Anything that must hold on every start
  cannot go in `init` (Facts for QS03, stale extraction); where package installation
  goes is QS04's decision.
- **A file the guest creates in the worktree** is the maintainer's on the host, mode
  0600 (claim 5).
- **One machine per state root** while it mounts the worktree (claim 16).

### 8. Facts for QS05

- **The endpoint.** `http://127.0.0.1:<random port>/operator` inside the guest, MCP
  streamable HTTP answering in server-sent events, `serverInfo` `rmcp 3.1.4`, 20 tools
  (claim 15). It is bound to guest loopback and its port changes at every `serve`
  start, so `-p` cannot publish it: `-p` needs a fixed port and a server on `0.0.0.0`.
  smolvm's `--auto-ports` is documented to publish guest-loopback listeners at the
  same port on host loopback; it was not tried, and the port would still be random.
  Nothing listens on 7001.
- **The route that works.** A stdio MCP server whose command, on the host, is
  `smolvm machine exec -i --name <machine> -w <worktree> -e OWD=<worktree> -- /opt/quint-studio/squashfs-root/AppRun mcp`,
  run with the same environment and working directory as every other smolvm command
  (Facts for QS02). `initialize` and `tools/list` answered through it; a session took
  about 0.2 s to open and `tools/list` 1 to 3 ms (claim 11). It needs the machine up
  and `serve` running with the worktree as its workspace.
- **What `connect claude` writes** (claim 10), as far as a run against a stand-in
  showed: it calls
  `claude mcp add --scope local quint-studio -- <path to quint-studio> mcp`, then
  `claude plugin marketplace list --json`, and by then has written 7 files under
  `.claude/skills/quint-studio/` in the repository and a plugin marketplace of 14
  files under `~/.local/share/co.quint.studio/claude-marketplace/`. Claim 10 lists
  all 21. The registration was only attempted: a stand-in `claude` recorded both
  calls and changed nothing, and `connect claude` then exited 1 on the stand-in's
  empty answer. So this is what Studio wrote before that failure, not the write set
  of a completed connection. What a real `claude` writes for `mcp add`, and what
  Studio does after a real marketplace listing, were not seen. Its command is a guest
  path, so what it would register cannot work on the host as written; and `.claude/`
  is masked read-only in the layout above, so `connect` run in the guest could not
  write the skill there.
- **Credential.** The endpoint answered an `initialize` that carried none, and no
  option in `serve --help` or `connect claude --help` names one. No Studio
  documentation was found to check (claim 15). That is the observed build as it was
  started here, not a proof that nothing can require one. The only gate seen is who
  can reach guest loopback or run `machine exec`.

### 9. What was written outside the worktree, and its removal

**By the spike's smolvm: nothing that any check found.** No "Authorisation required"
stop was needed in the end, so no path outside the worktree was allowed (Departures 1
and 2 say how that came about). Three kinds of evidence, each with its limit.

*The write boundary.* Every smolvm command ran through one wrapper,
`ai_tmp/qs01/bin/smolvm-qs01`: `cd` to the worktree root, `env -i` with `PATH`, `HOME`,
`TMPDIR` (both inside `ai_tmp/qs01/`), `TERM`, `DOCKER_CONFIG` (an empty directory) and
the relative `SMOLVM_DATA_DIR=ai_tmp/qs01/state`, then
`/usr/bin/sandbox-exec -f <profile> <the release's smolvm launcher> "$@"`. The profile,
`deny.sb`:

```text
(version 1)
(allow default)
(deny file-write*)
(allow file-write* (subpath "/Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp/qs01"))
(allow file-write*
  (literal "/dev/null")
  (literal "/dev/zero")
  (literal "/dev/tty")
  (literal "/dev/ptmx")
  (literal "/dev/dtracehelper")
  (regex #"^/dev/ttys[0-9]+$")
  (regex #"^/dev/fd/[0-9]+$"))
```

For the machines that mounted the worktree read-write, a second profile differs in one
line, allowing `file-write*` under the whole worktree and nowhere else. The triage
subagent's controls on both, before any boot under them: a write under the allowed
directory succeeded, and writes to `$HOME`, `/private/tmp`, the real `$TMPDIR`,
`/Users/Shared` and the worktree root (for `deny.sb`) or the worktree's parent (for the
second profile) were each refused and left nothing. The limit: the very first boot
attempt ran before this profile existed, under an earlier one that denied only `$HOME`,
`/private/tmp`, `/private/var/folders`, `/private/var/tmp`, `/Library`, `/usr/local` and
`/opt` (Departure 1).

*The kernel's record of refused writes.*
`/usr/bin/log show --style compact --predicate 'sender == "Sandbox" AND eventMessage CONTAINS "smolvm"'`,
exit 0, over the spike's whole run, is quoted with step 14's output below. Read at
15:35 local for the 90 minutes before, it held three lines, all from that first attempt
(pid 83346, 14:22:08), which are also the positive control that the log records such
refusals:

```text
Sandbox: smolvm-bin(83346) deny(1) file-write-create /private/tmp/ks-3sP6RA
Sandbox: smolvm-bin(83346) deny(1) file-write-create /private/var/folders/.../T/krun-eventfd-Ull69D
Sandbox: smolvm-bin(83346) deny(1) file-write-create /private/tmp/ks-arDtZJ
```

So smolvm tried to write outside the worktree three times, all refused, all before the
relative state root and the in-worktree `TMPDIR`; and under the final profile it tried
nothing that was refused. The limit: the log shows refusals, not writes the earlier
profile allowed.

*Step 3's probe, first run*, against `ai_tmp/qs01/marker` (made at 14:21:23 local,
before the binary first ran):

```text
find exit 1
root /Users/scutting maxdepth-1 entries 73
root /var/folders/x3/_1vnt8qn6t35q6gngl9fn3s00000gn/T/ maxdepth-1 entries 5065
root /var/folders/x3/_1vnt8qn6t35q6gngl9fn3s00000gn/C/ maxdepth-1 entries 369
root /private/tmp maxdepth-1 entries 65
root /Library maxdepth-1 entries 67
root /usr/local maxdepth-1 entries 2
/Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp/qs01/control-new-first
/Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp/qs01/control-old-first
```

The six `root` lines show less than a listing with a status would. `bin/probe.sh`
lines 12 to 14 run `find <root> -maxdepth 1 2>/dev/null | wc -l`: the count includes
the root's own path, `find`'s errors are dropped and its status is not kept. A count
of 2 or more means the root listed at least one entry beside itself, which all six
did. It does not show that the listing ended without an error.

It printed 1672 paths and 447 lines on stderr, 444 `Operation not permitted` and 3
`Permission denied`. With the maintainer's leave (2026-11-25) they are grouped here and
not quoted line by line; the file is `ai_tmp/qs01/probe-first.err`. Each is a directory
the probe could not read, and so **unprobed, not unchanged**:

```text
229 ~/Library/Containers/*
 92 ~/Library/Group Containers/*
 60 under $TMPDIR and the user cache directory (/var/folders/.../T and /C)
 12 ~/Library/Application Support/*
 10 ~/Library/Caches/*
  2 ~/Library/Metadata/*
  1 each: ~/Pictures/Photos Library.photoslibrary, ~/Music/Music, ~/Movies/TV, ~/Downloads, ~/.Trash,
    ~/Library/{Weather,Trial,Suggestions,StatusKit,Shortcuts,Sharing,Safari,Photos,PersonalizationPortrait,
    Mobile Documents,Messages,Mail,IntelligencePlatform,IdentityServices,HomeKit,DuetExpertCenter,DoNotDisturb,
    Daemon Containers,CoreFollowUp,Cookies,ContainerManager,com.apple.aiml.instrumentation,CloudStorage/Dropbox,
    Biome,Autosave Information,Assistant/SiriVocabulary,AppleMediaServices,Accounts},
    /Library/{Trial,Bluetooth,Caches/com.apple.aneuserd,Caches/com.apple.aned,
    Caches/com.apple.amsengagementd.classicdatavault,Application Support/com.apple.TCC}
  3 Permission denied: /Library/Application Support/Apple/ParentalControls/Users,
    /Library/Application Support/Apple/AssetCache/Data, /Library/Caches/com.apple.iconservices.store
```

None of the 447 names smolvm (`grep -c -i smolvm probe-first.err` printed `0`, exit 1;
its control, `grep -c 'Library/Caches' probe-first.err`, printed `14`, exit 0). Both
control files are among the 1672 paths (`probe-first.txt` lines 1350 and 1351).
Outside the worktree, the 1672 paths hold two that name smolvm,
`~/Library/Caches/smolvm/vms/0d1907069827eaec/storage.raw` and `.../egress`
(`probe-first.txt` lines 333 and 334). That machine was there before the spike. The
names-only listings taken at 14:18:19, before the marker (14:21:23) and the first boot
(14:22:08), name that directory and both files (`preexisting-vms.txt`
line 1; `preexisting-smolvm-names.txt` lines 111, 120 and 127). A
`smol-vmm _boot-vm .../vms/0d1907069827eaec/boot-config.json` process was running then,
pid 40635, from another project's Python environment
(`.../smol_browser/.venv/lib/python3.12/site-packages/smol/smol-vmm`,
`preexisting-smol-procs.txt` line 2), not from the spike's binary. And the spike's
smolvm ran under profiles that deny a write there: the first denied `$HOME`,
`deny.sb` denies everything outside `ai_tmp/qs01/` (lines 9 and 10 of the file), and
the second profile above denies everything outside the worktree. That
the other machine wrote the two files is an inference from those three facts: the
probe names no writer, and the process was seen at 14:18, not at probe time. The rest
outside the worktree is this session's own harness
(`~/.claude`, `/private/tmp/claude-501`), the reviewer's (`~/.codex`), Docker Desktop,
and ordinary macOS activity; `/private/tmp/` itself is listed, with `cc-socks` and no
`ks-` name. A further `find` over `/Users/Shared`, `/opt`, `/private/var/tmp`,
`/private/etc` and `/Volumes`, the roots the first profile left open, printed nothing
(exit 1, one unreadable directory, `/private/etc/cups/certs`, `probe-extra.err` line 1).

What the probe cannot see, as step 3 says: a write outside its roots, a file deleted
from a directory it lists, anything under the 447 directories it could not read, and a
write made behind it while it walked. That last limit applied. `bin/probe.sh` made the
two controls and started `find` at 14:48:57 local, after the first three boots
(14:22:08, 14:48:32 and 14:48:41), and `find` ended at 14:51:33; the times are those of
`control-new-first`, `probe-first.txt` and the logs under `logs/`. The later boots of
step 4 (14:49:25 to 14:49:39), the package install (to 14:51:00) and the first Studio
runs (14:51:24) happened while `find` was still walking, so it saw their writes only in
directories it had not yet passed. What came after 14:51:33 it did not see at all.

Step 14 reruns the probe against the same marker and lists the lines the first run
lacked. That comparison is of path names, and it is bounded in the same ways. A path
the first run already lists gives the same line when it is written again, so the
change does not appear: `probe-first.txt` already holds `/private/tmp` (line 1643),
the shared git object directory (line 5) and this worktree's entry in the common git
directory (line 6), among its 1672. The rerun cannot see under the directories it
cannot read, or a write made behind it as it walks, and its root counts have the
limit given above. So a rerun with no new line outside the worktree shows that no new
path name appeared in the directories it could read. It does not show that nothing
there was written. For the time after 14:51:33 the stronger evidence is the write
boundary and the refusal log above, not the probe.

**By the spike, not through smolvm**, each outside the worktree:

- **The clipboard.** With the maintainer's choice of 2026-11-25, the token URL was put
  on the clipboard with `pbcopy`, never printed, for a private browser window. It was
  cleared afterwards (`pbcopy < /dev/null`; `pbpaste | wc -c` printed 0), and `serve`
  has been restarted since, after which an old token is refused (claim 5).
- **The maintainer's browser** held the token cookie in a private window. The token
  changes at every `serve` start, and the machine has been stopped more than once and
  `serve` started again since (claim 8). An old token and cookie are refused after a
  restart: on `qs01-s`, a token read before a stop and start answered 200 before it
  and 401 after it, as the query and as the cookie
  (`logs/old-token-after-restart.txt` lines 4 to 8 and 14 to 18, quoted under claim
  5). That test
  used a token read for it. The browser's own cookie was not replayed, and it is older
  than the restart tested. Whether the window has been closed is the maintainer's to
  say. The spike did not close it, step 14 has no browser step, and these notes do not
  record it as closed. It is named here as remaining, with a cookie that by that test
  no longer opens anything.
- **One file in the real `$TMPDIR`**, written by mistake by the round-1 triage
  subagent and removed by it at once (`rm` exit 0, `test ! -e` exit 0, in its
  verdicts file).
- **Two listeners**, `python3 -m http.server` on `127.0.0.1:18765` and `<LAN>:18766`,
  serving an empty directory under `ai_tmp/qs01/` for step 10, killed after it
  (`lsof` on both ports: exit 1).
- **The reviewer and the harness** write their own session files under `~/.codex` and
  `~/.claude`, and the harness its scratch files under `/private/tmp/claude-501`. They
  are records of the sessions, not of smolvm or Studio. Step 14 does not remove them:
  they remain by design.
- **The shared git metadata.** This worktree's entry in the common git directory,
  `/Users/scutting/projects/pachinko/.git/worktrees/QS01`, and the objects and refs
  this ticket's commits add to that directory are outside the worktree. They are the
  repository itself and remain by design.
- **Network**: the release and its checksum from GitHub, the source at the tag and its
  libkrun commit, the registry APIs, Docker Hub, Debian's mirrors, Studio's download,
  and whatever Studio itself contacted once signed in (its upstream, a telemetry queue,
  one model provider).

**What remains outside the worktree after step 14, by design:** the private browser
window, if it is still open; the reviewer's and the harness's files under `~/.codex`,
`~/.claude` and `/private/tmp/claude-501`; and the shared git metadata. Each is named
above. The acceptance criterion asks for the maintainer's permission for what
remains, and no answer for these is recorded yet.

**Inside the worktree, to throw away**, all under `ai_tmp/`, which Git ignores: the
state directory (machines, an image seed, a registry pull token smolvm cached, never
read), the unpacked release with its two generated templates, the AppImage, the
sources, logs and scripts. Step 14 deletes the machines and removes the state
directory. **In the guest, sign-in lives only on the machine's disk** and goes with
it. The browser's cookie is listed above, and the Quint account's session and the two
provider registrations on Studio's side are the maintainer's to revoke if they wish.

Step 14's output is appended here.

#### Step 14's output

Run after checkpoint 4's second round, from 18:57 to 19:01 local, at `c5ca91f` with
uncommitted changes (this ticket's notes). The first three blocks are
`logs/cleanup.txt`, in its order.

*Machines.* One machine was left, `qs01-s`, stopped. smolvm 1.37.4 has no `volume`
subcommand (`smolvm volume --help`: `error: unrecognized subcommand 'volume'`), and
`qs01-s` had no disk volume (`bin/s-create.sh`), so there was no volume to delete
beside it.

```text
$ smolvm machine ls
NAME   STATE         CPUS     MEMORY  MOUNTS   PORTS  STORAGE  OVERLAY  BRANCHED FROM
-------------------------------------------------------------------------------------
qs01-s stopped          4   8192 MiB      12       1   20 GiB   10 GiB  -
ls exit 0
$ smolvm machine delete -f --name qs01-s
Deleted machine: qs01-s
delete exit 0
$ smolvm machine ls
No machines found
ls exit 0
$ smolvm machine ls --json
[]
ls --json exit 0
$ pgrep -fl QS01/ai_tmp/qs01/smolvm
pgrep exit 1 (1 = no process)
```

*The state directory, and the token files.* The state directory went, as said above.
So did the four files that held `serve` tokens and one saved page fetched with a
token, all under `ai_tmp/qs01/` and all dead since the machine's last stop; none was
printed. `test ! -e` on each, with a control on a path that exists:

```text
$ du -sh ai_tmp/qs01/state; ls -1 ai_tmp/qs01/state
181M ai_tmp/qs01/state
Library
qs01-host-file
$ rm -rf ai_tmp/qs01/state
rm exit 0
$ rm ai_tmp/qs01/token ai_tmp/qs01/token-1 ai_tmp/qs01/token-A ai_tmp/qs01/token-B ai_tmp/qs01/ui-with-token.html
rm exit 0
test ! -e ai_tmp/qs01/state: exit 0
test ! -e ai_tmp/qs01/token: exit 0
test ! -e ai_tmp/qs01/token-1: exit 0
test ! -e ai_tmp/qs01/token-A: exit 0
test ! -e ai_tmp/qs01/token-B: exit 0
test ! -e ai_tmp/qs01/ui-with-token.html: exit 0
$ test ! -e, control on a path that exists
test ! -e ai_tmp/qs01/marker: exit 1
```

The commands ran with each path written out in full under the worktree; the log
prints them relative to it. The rest of `ai_tmp/qs01/` stays for the maintainer to
read against these notes: the unpacked release, the AppImage, the sources, the logs
and the scripts. Git ignores all of it, and removing `ai_tmp/qs01/` removes it.

*Outside the worktree.* Step 3 and step 4 were allowed no path outside the worktree,
so there is none of theirs to remove. The three paths the first boot was refused are
the nearest thing to a list, and none exists. No `ks-` name is in `/private/tmp`, and
no `krun-eventfd-` name is in the real `$TMPDIR`. Port 7000 is held only by the
system's AirPlay receiver again (`ControlCe` is `lsof`'s cut of `ControlCenter`):

```text
test ! -e /private/tmp/ks-3sP6RA: exit 0
test ! -e /private/tmp/ks-arDtZJ: exit 0
$ ls /private/tmp | grep -c "^ks-"
0
grep exit 1 (1 = no match)
$ ls /private/tmp | grep -c "^claude-"   (control: the listing is read)
1
grep exit 0
$ find "$TMPDIR" -maxdepth 1 -name "krun-eventfd-*" | wc -l
       0
$ lsof -nP -iTCP:7000 -sTCP:LISTEN (command and address only)
COMMAND NAME
ControlCe *:7000 (LISTEN)
ControlCe *:7000 (LISTEN)
lsof done
test ! -e /private/var/folders/x3/_1vnt8qn6t35q6gngl9fn3s00000gn/T/krun-eventfd-Ull69D: exit 0
```

In the log the two `ks-` tests sit under a heading that says three paths; the third
is the last line, added a few minutes later. The `find` for `krun-eventfd-*` has no
positive control and its status was not kept. Trailing spaces and one tab in the
log's lines are not reproduced.

*The kernel's record of refused writes, over the whole run.*
`/usr/bin/log show --style compact --start '2026-11-25 14:20:00' --predicate 'sender == "Sandbox" AND eventMessage CONTAINS "smolvm"'`,
read at 19:01:40 local, exit 0, saved as `logs/sandbox-denials-final.txt`: a header
line and the same three lines as at 15:35, and no other.

```text
2026-11-25 14:22:08.122 E  kernel[0:229a5958] (Sandbox) Sandbox: smolvm-bin(83346) deny(1) file-write-create /private/tmp/ks-3sP6RA
2026-11-25 14:22:08.446 E  kernel[0:229a5958] (Sandbox) Sandbox: smolvm-bin(83346) deny(1) file-write-create /private/var/folders/x3/_1vnt8qn6t35q6gngl9fn3s00000gn/T/krun-eventfd-Ull69D
2026-11-25 14:22:08.446 E  kernel[0:229a5958] (Sandbox) Sandbox: smolvm-bin(83346) deny(1) file-write-create /private/tmp/ks-arDtZJ
```

So from the second boot to the last command, the deletion included, the spike's smolvm
attempted no write that its profile refused.

*The probe, rerun* against the same marker with `sh ai_tmp/qs01/bin/probe.sh final`,
from 18:58:21 to 19:00:53 local, after the deletion and the removal:

```text
find exit 1
root /Users/scutting maxdepth-1 entries 73
root /var/folders/x3/_1vnt8qn6t35q6gngl9fn3s00000gn/T/ maxdepth-1 entries 5094
root /var/folders/x3/_1vnt8qn6t35q6gngl9fn3s00000gn/C/ maxdepth-1 entries 369
root /private/tmp maxdepth-1 entries 65
root /Library maxdepth-1 entries 67
root /usr/local maxdepth-1 entries 2
/Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp/qs01/control-new-final
/Users/scutting/.supacode/repos/pachinko/QS01/ai_tmp/qs01/control-old-final
```

Both controls are in its output (`probe-final.txt` lines 2029 and 2030). Its stderr
is the first run's, byte for byte (`cmp probe-first.err probe-final.err`: exit 0), so
the same 447 directories were unprobed and no other. It printed 10467 paths.
`comm -13` over the two sorted lists gives 8858 the first run lacked: 225 inside this
worktree and 8633 outside it (`probe-new-outside.txt`). None is the parent of a
removed path, since nothing outside the worktree was removed. The 8633 are grouped
here and not listed one by one, as the first run's stderr was; the maintainer's
leave of 2026-11-25 was given for that file, not for this list, so this grouping is
the lead's own choice and one more departure. `logs/probe-final-groups.txt`, whose
counts add up to 8633:

```text
 5481 ~/.supacode/repos/pachinko/mk-quint-spec-tickets (another worktree)
    5 ~/.supacode/repos/pachinko/build-gleam-double (another worktree)
  461 ~/.supacode/repos/transect/* (another project)
    2 ~/projects/pachinko/.git/worktrees/{build-gleam-double,mk-quint-spec-tickets}
  110 ~/projects/biscuitbin/* (other projects)
 1388 $TMPDIR/pytest-of-scutting
   73 $TMPDIR, other
   41 the user cache directory
  302 ~/.cache/uv
    7 ~/.cache/prek
   74 ~/.claude
   68 ~/.codex
   87 /private/tmp/claude-501
    3 /private/tmp/codex-browser-use
    1 ~/Library/Caches/smolvm/vms/0d1907069827eaec/agent-startup-error.log
    7 ~/Library/Caches/rattler
  451 ~/Library, other
   45 ~/Documents/superwhisper
   11 ~/.cargo, ~/.npm, ~/.gnupg, ~/.local
   16 /Library
```

Each group is activity that is not the spike's smolvm, its guest or Studio, for one
reason common to all and one of its own. The common reason: since the second boot,
every smolvm process of the spike's ran under a kernel profile that refuses a write
outside `ai_tmp/qs01/` or, for the machines that mounted the worktree read-write,
outside the worktree, and the refusal log above holds nothing after the first boot.
The reasons of their own:

- **Other worktrees and projects** (the first five lines, and the `pytest` and `uv`
  lines): other sessions were at work on this machine through the afternoon, in two
  other worktrees of this repository and in other projects. Their trees, their
  worktree entries in the common git directory, their test runs' temporary
  directories and their package cache are theirs.
- **This session's own tools, not smolvm:** `~/.claude` and `/private/tmp/claude-501`
  (the harness, for this session and the others), `~/.codex` (the reviewer's sessions,
  and the maintainer's own use of Codex, its dictation history among it),
  `~/.cache/prek` and `~/Library/Caches/rattler` (the caches of this repository's hook
  gate and of pixi, which `just lint` and `just check-docs` wrote in this worktree
  and other worktrees' gates wrote too). The first three are named above as remaining
  by design. The two caches are the tooling's own and are not this ticket's to remove.
- **One line names smolvm:** `agent-startup-error.log` in the directory of the machine
  `0d1907069827eaec`, last written at 17:08:53. That machine is the other project's,
  as the first run's two lines there were: its `smol-vmm` process, pid 40635, from
  that project's Python environment, was still running at 18:57. The spike's smolvm
  ran under profiles that refuse a write there, and no refused attempt is in the log.
  The file was not opened.
- **The rest** (`~/Library`, the user cache directory, `$TMPDIR`, `/Library`,
  `~/Documents/superwhisper`, and the four dot-directories) is the machine in use:
  Chrome, the Codex and Claude desktop applications, Docker Desktop, Apple's own
  services, a dictation tool, and `cargo`, `npm`, `gpg` and an editor run by this or
  another session. That reading is from the path names; no writer was traced.
  `grep -c smolvm probe-new-outside.txt` printed `1`, the line above.

The limits given above for this comparison hold: it is of path names, in the
directories the probe could read.

*The maintainer's permission for what remains* (2026-11-25, asked after the clean-up).
Asked whether the harness's and the reviewer's files under `~/.claude`, `~/.codex` and
`/private/tmp/claude-501`, the shared git metadata and the private browser window, if
still open, may stay, the maintainer answered "Yes, they may stay". Asked whether the
private window used for Studio is closed, the maintainer answered "Closed". So the
sentence above that no answer is recorded is answered here, and the browser window no
longer remains.

### 10. Checkpoint records

**Harness and model**, for every checkpoint that ran: Codex CLI 0.172.0,
`codex exec -m gpt-6.2-sol -s read-only`, in the form Departure 12 quotes, one run for
each lens, conformance and breakage. No fallback model was used. Triage and apply were
done by subagents that had neither written the artefact nor reviewed it; Departure 10
says what the lead applied itself. The raw findings, verdicts and logs are under
`ai_tmp/review/QS01-<n>/` and are not committed. Every run was at `c5ca91f` with
uncommitted changes (this ticket's notes).

#### Checkpoint 1

Over `ai_tmp/review/QS01-1/artefact.md`, the list step 3's stop would ask for: at
first a request to allow `/private/tmp/ks-*`, with the wrapper, the profile and a
removal loop. Round 1 had valid high findings, so a second round ran over the fixes,
the last §14 allows.

**Round 1.** 17 findings, 13 distinct by fix (C02 = B01, C04 = B06, C05 = B07,
C07 = B09). Seven were high (C01, C03, C04, C05, B02, B06, B07). 17 valid, 0 invalid.

- C01 (high), valid: a boot had already been attempted before the stop
  (`logs/boot1.txt`). Recorded in the artefact; the Departures entry was handed back
  (Departure 1).
- C02/B01, valid: the artefact measured only the absolute state root. A relative
  `SMOLVM_DATA_DIR` gives socket paths of 71 and 78 bytes. The wrapper now sets it,
  from the worktree root.
- C03 (high), valid: the profile allowed by default and denied a finite list, leaving
  `/Users/Shared` open, and the proposed `ks-` rule matched longer names. `deny.sb` now
  denies every file write and allows only `ai_tmp/qs01/` and device nodes.
- B02 (high), valid: an inherited `SMOLVM_USAGE_DIR` could send the usage journal
  outside the worktree. The wrapper runs smolvm through `env -i` with six named
  variables. That the probe's roots omit `/Users/Shared` was handed back.
- B03, valid: libkrun also writes `$TMPDIR/krun-eventfd-*`, which the denied `$TMPDIR`
  hid. `TMPDIR` now points inside the worktree.
- B04, valid: an empty `machine ls` does not prove the redirect. The artefact rests it
  on the files seen under `state/`.
- B05, valid: the `pkgutil` receipt is not the file smolvm tests for. The `test -e` on
  the runtime file is quoted beside it, exit 0.
- C04/B06 (high), valid: the removal loop never tested the outer `ks-*` entry and its
  glob matched other names. Replaced by a script that acts only on entries absent from
  an inventory taken before.
- C05/B07 (high), valid: a kill between the directory's creation and its symlink leaves
  an empty directory the loop skipped, and a later iteration masked a failed removal.
  The script removes the empty directory and checks each removal on its own.
- C06, valid: no probe had run, and the `ks-*` listing showed no status and no control.
  Corrected in the artefact; running the probe was handed back (section 9).
- C07/B09, valid: the profile denied the worktree writes steps 6 and 9 need. A second
  profile, `deny-worktree-rw.sb`, allows writes under the worktree and nowhere else.
- B08, valid: an alias can exist too briefly for any listing, and a dangling symlink
  passes `test ! -e`. Removal tests `-e` and `-L`, and the limit is stated.
- B10, valid, not applied: the token URL would stay in the host browser's history.
  Handed back (Departure 5).

**Round 2** (the last). 19 findings (C2-01 to C2-08, B2-01 to B2-11). Eight were high
(C2-01, C2-02, C2-03, B2-01, B2-02, B2-04, B2-06, B2-09). 19 valid, one of them, B2-06,
in one of its two scenarios only; 0 invalid.

- C2-01/B2-01 (high), valid: `QS01_SB` took any file as the profile. It now names one
  of the two profiles and never carries a path. The second half of C2-01 went with
  B2-04.
- B2-02 (high), valid: a bare `mkdir` ran before `env -i`, and `/bin/sh` here imports
  exported functions. `PATH` is set first, every external command is an absolute path,
  and `cd` is cleared with `unset -f`.
- B2-03, valid: `--state-dir` would make the root absolute again. The wrapper refuses
  it before a literal `--`.
- C2-02 and B2-06 (both high) and B2-07, valid, not applied: the removal script cannot
  prove who made an empty alias directory, and a name holding a newline defeats its
  comparison. The fallback was never used (Open point 12).
- B2-04 (high), valid, handed back: a guest with the worktree mounted read-write could
  rewrite the wrapper and both profiles. `ai_tmp` is among the read-only masks of the
  layout (claim 13, Facts for QS04), which were tested before step 6 for this reason
  (Departure 4).
- C2-03 (high), valid, handed back: the first boot preceded the checkpoint, under a
  default-allow profile. Departure 1 and section 9 record it and its limit.
- B2-09 (high), valid, handed back: step 14 lists only new lines, so a path the first
  probe already lists can change unseen. Section 9 states the probe's limits; step 14's
  own output is pending.
- C2-04, valid, handed back: the probe exited 1 with 447 unread directories. Section 9
  groups them (Departure 9) and says when the controls were made. No rerun of the
  alias listing with a control and statuses is in these notes; section 9 gives the
  probe's listing of `/private/tmp` in its place.
- C2-05, valid, handed back: no transcript backed the named-machine commands. The
  later ones have saved logs under `ai_tmp/qs01/logs/`; `machine data-dir`'s status is
  not among them (claim 2). Claim 2 is scoped to what was seen, and volumes are under
  claim 16.
- C2-06/B2-10, valid, handed back: the token URL in the browser. The agent did not
  open it; the maintainer pasted it into a private window (Departure 5, section 9).
- C2-07, valid, handed back: the state directory holds a registry pull token. It was
  never read; section 9 lists it to throw away, and step 14's removal is pending.
- C2-08 (low), valid, handed back: a triage subagent wrote one file in the real
  `$TMPDIR` and removed it. Section 9 records it.
- B2-05, valid, handed back: `-p 127.0.0.1:7000:7000` cannot parse. Claim 5 records the
  form that works and the `lsof` listing. It is not listed under Departures, and a
  search of the machine's log for `control socket unavailable` is not recorded.
- B2-08, valid, handed back: `machine delete` can report success without deleting, so
  each deletion is to be proved by listing and by path. That is step 14's, pending.
- B2-11, valid, handed back: a failed commit does not show which layer refused it.
  Claim 7 quotes the guest's `ro` mount and the `Read-only file system` error.

**Applied.** Round 1: the wrapper `ai_tmp/qs01/bin/smolvm-qs01`, the two profiles and
the artefact. Round 2: the wrapper only. All of it is scratch under `ai_tmp/`.

**Re-run.** After round 1, at `c5ca91f` with uncommitted changes: the boot under
`deny.sb` with the relative state root, which printed `x86_64` (claim 2 records it).
With that boot nothing was left to ask for, and the stop was not made (Departure 2).
Round 2's fixes changed only the wrapper's guards; no run was quoted for them alone,
and the machine commands after them ran through that wrapper (section 9).

**Left open.** Two rounds is the limit. Open point 12 holds the unused fallback.
Step 14's parts of C2-07, B2-08 and B2-09 wait for step 14.

#### Checkpoint 2

Did not run. Rosetta was already installed: `pkgutil` exited 0 and the runtime file
smolvm tests for exists (claim 3), so step 4's stop was never reached.

#### Checkpoint 3

Over a snapshot of these notes as they stood at 15:14 local,
`ai_tmp/review/QS01-3/artefact-ticket-snapshot.md`, 925 lines. It ran after step 6 had
begun, not at the end of step 5 (Departure 4).

**Round 1.** 15 findings (C01 to C08, BR-01 to BR-07). Two were high (BR-04, BR-05).
12 valid, 3 valid in part (C03, BR-02, BR-07), 0 invalid.

- C01, valid: claim 2 deferred the probe's limits to a section not yet written, and
  did not quote the evidence for whose machine the two outside paths are. Section 9 now
  carries the probe's timing against the boots, the listings and the process, the
  `grep` controls and the limits; claim 2 points to it.
- C02/BR-01, valid: the usage journal is under `Library/Application Support/smolvm/`,
  not at the state root; another working directory gives a second state tree, not the
  default one; and the default paths are upstream's, not observed. Corrected in claim 2
  and Facts for QS02.
- C03, valid in part: `machine data-dir` had no exit line and the digest block no
  command. The notes did name the commit once for every quote. The digest block now
  shows its commands and statuses, `data-dir`'s status is stated as not recorded, and
  the blocks are headed by the commit.
- C04, valid: the two `--rosetta` logs hold no outer exit line, and smolvm does not
  drop `--rosetta` "silently": it still selects the amd64 image and no command refuses
  first. Claim 3 corrected, with the saved transcript of the three runs.
- C05, valid: "no FUSE" was contradicted by the guest, which has `/dev/fuse`; the
  extraction's own output was not saved. Claim 4 now says neither method mounts
  anything, quotes the FUSE and display checks with their control, the rerun of
  method 2 and method 1 reaching `serve`.
- C06/BR-05 (BR-05 high), valid: the stamp had no rule for a missing stamp or a failed
  re-extraction, and `init` runs only on the first start. Facts for QS03 states the
  fail-closed rule and where the check must live; claim 4 cites the restart evidence.
- C07, valid: the 15 library names are in no saved log, and the saved `ldd` summary
  contradicted itself. Claim 4 says so, quotes the later `ldd` transcript with its
  scope, and rests sufficiency on the machine provisioned from the bare image.
- C08, valid: claim 16's step-4 half was pending. It was written by the lead before
  these fixes; the help lines and the making of `qs01-s` are now cited.
- BR-02, valid in part: the socket rule is length, not relativeness. Stated in Facts
  for QS02. The wrapper and profiles name this worktree's path, but are scratch that no
  ticket inherits.
- BR-03, valid: a plain template is preferred over any archive and never refreshed.
  Stated in Facts for QS02, read from the source.
- BR-04 (high), valid: the image pull needs `--net`, which is off by default. Stated in
  Facts for QS04 with the failing log, and in the flags list of Facts for QS02.
- BR-06, valid: no saved run showed the interpreter on a non-executable file. A
  mode-0644 copy ran through it, exit 0; a 0644 file on the read-only mount itself was
  not run. Stated in Facts for QS03.
- BR-07, valid in part: the notes already said the install is unpinned. Where each
  version came from is now recorded for the first install, and stated as not recorded
  for the final machine's 89.

**Applied** by the apply subagent, to these notes only: fixes F1 to F13 of
`ai_tmp/review/QS01-3/verdicts.md`, each adapted to the notes as they then stood. No
machine command and no network were used. Of the four transcripts triage asked for,
the lead captured one in another form: the restart, on `qs01-s`
(`logs/persistence.txt`, claim 8). Three were not captured, and the notes carry the
caveat in each place: the banners beside their commands (claim 3 matches them by
count), `ldd` before and after the install on a new machine (claim 4), and a mode-0644
AppImage on the read-only mount (Facts for QS03). `machine data-dir`'s exit status was
not captured either (claim 2).

**Handed back.** To QS02: its own wrapper and profiles (BR-02), and the rule for
installing a release into a new directory (BR-03). To QS03 or QS04: building the stamp
check (BR-05). To QS04: an install-tested snapshot timestamp or image digest (BR-07,
Open point 8).

**Re-run.** No machine was run for these fixes. The transcripts taken after the
findings are quoted where the fixes cite them (claims 3 and 4). After the fixes, at
`c5ca91f` with uncommitted changes (this ticket's notes), `just check-docs`:

```text
$ just check-docs
prek run --all-files markdownlint-cli2 typos lychee
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
bg-validate-docs
Validated 49 pages and 50 canonical topics.
just exit 0
```

Its first run failed on the numbering of Open points 9 to 13, a second list that began
at 9 (MD029); the two lists are now one, with the same numbers.

**Round 2** (the last), run because round 1 had valid high findings, over the fix diff
between `ai_tmp/review/QS01-3/ticket-before-apply.md` and `ticket-after-apply.md`. 11
findings (C2-01 to C2-07, BR2-01 to BR2-04), 8 distinct by fix (C2-01 = BR2-01,
C2-02 = BR2-02, C2-06 = BR2-04). None was high. 9 valid, 2 valid in part (C2-04,
C2-07), 0 invalid.

- C2-01/BR2-01, valid: claim 2 still quoted `run exit 0` and a `data-dir` line that no
  saved log held. The lead ran both again with their statuses
  (`logs/rerun-statuses.txt`). Claim 2 quotes that rerun and gives the first boot and
  lookup in prose, with their sources. This supersedes the "not recorded" of round 1
  above and of checkpoint 1's C2-05.
- C2-02/BR2-02, valid: `--rosetta` asks for `linux/amd64`, which is not always what
  the machine gets, and a host without Rosetta need not fail at its first exec. Claim
  3 gives the three outcomes the source allows, none of them run.
- C2-03 (low), valid: the first `serve`'s command line was a reconstruction shown as a
  `$` line. Claim 4 gives it in prose as a partial reconstruction and quotes only the
  saved output.
- C2-04, valid in part: the notes named `<home>/.smolvm`, but the upgrade rule covered
  neither it nor the cache. Facts for QS02 says the rule must.
- C2-05 (low), valid: the machine with no network had no cite. Claim 16 cites `qs01-m`
  and its saved `uname`, and Facts for QS04 says how its image reached it was not
  recorded.
- BR2-03, valid: `--net` read as needed for every first pull. Facts for QS04 scopes it
  to `machine run`'s pull in the guest, and gives the fetch on the host as read from
  the source, not run.
- C2-06/BR2-04, valid: the stamp's hash comes from a second open of the mounted path.
  Facts for QS03 says extraction and stamp must come from one copy that cannot change
  between them.
- C2-07, valid in part: the notes listed `git` and `curl`, but read as if the smaller
  set had been tested. Claim 4 says it was not.

**Applied** by the triage-and-apply subagent, to these notes only, together with
checkpoint 4's round 1. It ran no machine command and used no network. The verdicts are
in `ai_tmp/review/QS01-3/verdicts-r2.md`.

**Handed back.** To QS02: what a host without Rosetta does (C2-02), and an upgrade rule
that covers the state root (C2-04). To QS03 or QS04: one verified copy for extraction
and stamp (C2-06). To QS04: whether a named machine needs `--net` for its image
(BR2-03), and the reduced package set (C2-07).

**Re-run.** No machine was run for these fixes other than the lead's rerun, quoted
under claim 2. `just check-docs` after both sets of fixes is quoted under checkpoint 4.

**Left open.** Two rounds is the limit. Every round-2 finding has its fix in these
notes. What those fixes state from the source and did not run is Open point 15.

#### Checkpoint 4

Over a snapshot of these notes as they stood at 18:12 local,
`ai_tmp/review/QS01-4/artefact-ticket-snapshot.md`, 2351 lines: drafted in full except
step 14's output and Verification.

**Round 1.** 22 findings (C01 to C11, BR-01 to BR-11), 15 distinct by fix (C03 = BR-03,
C05 = BR-05, C06 = BR-02, C08 = BR-04, C09 = BR-09, C10 = BR-11, C11 = BR-10). Three
were high (C01, BR-01, BR-06). 19 valid, 3 valid in part (C09, BR-08, BR-09), 0
invalid.

- C01 (high), valid: section 3 read as a go confirmed by the maintainer, while claim
  13's `.git` conflict has no tested workaround and Open point 1 has no ruling. Section
  3 now says go for the mechanism, with QS04 conditional on that ruling, and records
  the maintainer's answer as leave to finish the spike.
- C02, valid: the same as checkpoint 3's C2-01, and fixed with it (claim 2).
- C03/BR-03, valid: the sign-in listing's `find exit 0` is `sort`'s status, and `-xdev`
  skips a filesystem mounted under a root. Claim 6 says both and keeps the paths.
- C04, valid: claim 8 showed the project's directory before `serve` ran again, not
  Studio answering for it. It now quotes a later start, `status` and three
  `read_project_summary` answers (`logs/project-after-restart.txt`).
- C05/BR-05, valid: no UI action was timed three times. Claim 11 says so and names
  what was measured (Departure 13).
- C06/BR-02, valid: "nothing can make it require one" went beyond one answer and two
  help texts, and the quoted `curl exit 0` was an `echo`'s status. Claim 15 and Facts
  for QS05 are reworded, the line is out of the quote, and the search for Studio's
  documentation, which found none, is recorded.
- C07, valid: mounts are not "listed only as counts". Claim 16 quotes
  `machine ls --verbose` and says a disk volume shows only in the guest's
  `/proc/mounts`.
- C08/BR-04, valid: `connect claude` ran against a stand-in and exited 1, and six of
  its 21 files were not named. Claim 10 and Facts for QS05 say the registration was
  attempted, list all 21 and state the listing's limits (Departure 14). Triage also
  found the failure line attached to the wrong run:
  `Claude Code MCP configuration failed:` belongs to the run with no `claude` on
  `PATH`.
- C09/BR-09 (low), valid in part: the `lsof` rows carried the account name, now
  `<user>`. The absolute paths stay, as in `tickets/T18-verification-gaps.md` and
  `tickets/CONVENTIONS.md`.
- C10/BR-11, valid: the probe's root counts can hide an error, and step 14's
  comparison of path names cannot show a second write to a path already listed.
  Section 9 bounds both.
- C11/BR-10, valid: the browser window, the harness's and the reviewer's files and the
  shared git metadata had no disposition. Section 9 names each as remaining and says
  no leave for them is recorded yet.
- BR-01 (high), valid: a machine's network policy is not fixed at creation. By the
  source, `machine update --no-net` clears the deny rules and `--net` does not restore
  them. Facts for QS02 says so, and that QS04 never toggles networking on an existing
  machine.
- BR-06 (high), valid: a read-only mask at the same path stops writes, not reads, so
  the guest could read the state directory and the rest of `ai_tmp/`. Claim 13 and
  Facts for QS04 say so (Open point 14).
- BR-07, valid: under the redirect a Smolfile's `$HOME` is the state root, and a
  missing mask source fails the create. Facts for QS04, read from the source.
- BR-08, valid in part: claim 8 gave the wrapper's `/Users/` condition, but not that it
  leaves a caller's own object directory alone. Added there and in Facts for QS04.

**Applied** by the triage-and-apply subagent, to these notes only: the corrections
above, Departures 13 and 14 and Open points 14 and 15. It ran no machine command and
used no network. The evidence captured after the reviews is the lead's:
`logs/rerun-statuses.txt`, `logs/project-after-restart.txt` and the four pages under
`ai_tmp/qs01/docs/`. The verdicts are in `ai_tmp/review/QS01-4/verdicts.md`.

**Handed back.** To the maintainer: Open points 1 and 14, and leave for what remains
outside the worktree (C11). To QS04: never toggling networking on an existing machine
(BR-01), an empty mask for the state directory (BR-06), the Smolfile traps (BR-07) and
the wrapper's conditions (BR-08). To QS05: `connect claude` against a real `claude`
(C08). Not applied: replacing the absolute paths (C09/BR-09), and the full content of
`connect claude`'s files, which stays in scratch (C08).

**Re-run.** The subagent that applied these fixes ran no machine. The runs the fixes
cite are the lead's, taken after the reviews and quoted under claims 2, 8 and 16. After
the fixes of both checkpoints, at `c5ca91f` with uncommitted changes (this ticket's
notes), `just check-docs`, from `ai_tmp/qs01/logs/check-docs-4.txt`. That run was made
before this paragraph was added, and the gate was run again after it:

```text
$ just check-docs
prek run --all-files markdownlint-cli2 typos lychee
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
bg-validate-docs
Validated 49 pages and 50 canonical topics.
just exit 0
```

`just check` is step 14's, on the commit.

**Round 2** (the last), run because round 1 had valid high findings, with
`codex exec -m gpt-6.2-sol -s read-only`, one run for each lens, over the fix diff
between `ai_tmp/review/QS01-4/ticket-before-apply.md` and `ticket-after-apply.md`. 9
findings (C4R2-01 to C4R2-04 from conformance, BR2-01 to BR2-05 from breakage; the
breakage ids repeat those of checkpoint 3's round 2 and name other findings), 7
distinct by fix (C4R2-02 = BR2-05, C4R2-03 = BR2-03). None was high. 8 valid, 1 valid
in part (BR2-04), 0 invalid.

- C4R2-01, valid: section 3 gave no ticket-level disposition and no fallback, which
  the Goal asks for. It now reads go, conditional, names the three things the
  maintainer can accept, and gives the no-go outcome with the Goal's fallback. Triage
  found two places where the lead's draft wording went past the files: QS02's decision
  and demo boot do carry ruling 2, and the README has QS02 to QS05 not start on a
  no-go. Section 3 says both.
- C4R2-02/BR2-05, valid: Facts for QS04 said the wrapper redirects only for the
  mounted common directory, while the script tests for a path that starts with
  `/Users/`. Facts for QS04 now describes the script as it is, with its destination,
  and says QS04's wrapper should compare with the exact mounted path. Claim 8 names
  the destination.
- C4R2-03/BR2-03 (low), valid: claim 15 said no file held the documentation statuses.
  It now quotes `logs/studio-docs-search.txt`, the control included.
- C4R2-04 (low), valid: section 9 said the old cookie had died, on the evidence that
  the token's value differed. Claim 5 quotes a token refused after a restart
  (`logs/old-token-after-restart.txt`), and section 9 cites it and says the browser's
  own cookie was not replayed.
- BR2-01, valid: by the source, an ephemeral `machine run` with a deny CIDR has its
  image fetched on the host, so "pulls the image inside the guest" was too wide. Facts
  for QS04 says what `logs/boot2.txt` shows and gives the host fetch as read from the
  source.
- BR2-02, valid: three answers after a restart do not show that Studio kept the map
  it built before. Claim 8 says what is shown, that the map was answered in 3 to 17 ms
  and no model call was made since the scan, and that the two maps were not compared.
- BR2-04 (low), valid in part: the notes already gave `shared_folders` as read from
  the source, and `machine status` without `--json` does name neither kind. Missing
  were the staged mount that `ls --verbose` counts and does not print, and that
  `status --json` carries the mappings. Claim 16 adds both, as read from the source.

**Applied** by the triage-and-apply subagent, to these notes only: the corrections
above, the correction to Departure 3 that the lead asked for, and Open point 16. It
ran no machine command and used no network. The two logs the fixes quote are the
lead's. `logs/studio-docs-search.txt` (18:40:23) was saved just before the round began
and both reviewers cite it. `logs/old-token-after-restart.txt` (18:44:19) was saved
after the round, so no reviewer saw the log that claims 5 and 8 now quote. Triage
found that its usage reading stops at `by project`, where the reading after the scan
went on, and claim 8 says so. The verdicts are in
`ai_tmp/review/QS01-4/verdicts-2.md`.

**Handed back.** To the maintainer: the conditional go of section 3, with Open
point 1. To QS04: a git wrapper that compares with the exact mounted path (C4R2-02),
whether a run with a deny CIDR pulls through the guest (BR2-01), and the limits of the
two mount listings (BR2-04). To QS04 or QS05: whether Studio keeps its project across
a restart (BR2-02).

**Re-run.** No machine was run for these fixes by the subagent. The lead's runs are
quoted under claims 5, 8 and 15. After the fixes, at `c5ca91f` with uncommitted
changes (this ticket's notes), `just check-docs`, from
`ai_tmp/qs01/logs/check-docs-4-2.txt`. That run was made before this record was added,
and the gate was run again after it:

```text
$ just check-docs
prek run --all-files markdownlint-cli2 typos lychee
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
bg-validate-docs
Validated 49 pages and 50 canonical topics.
just exit 0
```

**Left open.** Two rounds is the limit. Every round-2 finding has its fix in these
notes. What those fixes leave unchecked or unrun is Open point 16.

### 11. Departures

1. **A boot was attempted before checkpoint 1 and step 3's stop.** The first
   `machine run` ran at 14:22:08 with an absolute state root, under a `sandbox-exec`
   profile meant to make any write outside the worktree fail, which it did (claim 2,
   and the three refusals in section 9). The ticket puts the review and the stop before
   the first boot. That profile was also looser than it was taken to be: it allowed by
   default and denied seven named roots. Nothing was found written through that gap
   (section 9), which is weaker than a denial.
2. **Step 3's stop was reached, reviewed and then not made.** Checkpoint 1 ran over a
   request to allow `/private/tmp/ks-*`. Both lenses pointed at a relative
   `SMOLVM_DATA_DIR`; the fixed wrapper and a deny-all profile then booted with no
   alias, so nothing was left to ask for. §14 says an agent "still stops and asks" at an
   authorisation checkpoint; here it went on, and told the maintainer at the next
   stop, before sign-in.
3. **The lead's work overlapped a subagent's writing, four times.** Twice with boots:
   the lead waited for the round-1 and round-2 triage files to be still for 90 and 120
   seconds and then ran machines, while each subagent was in fact still finishing
   `verdicts.md` and, in round 1, the artefact. The wrapper and profiles they edited
   were final before any boot used them. No gate ran during either. It then happened a
   third and a fourth time in a milder form. The lead ran `just lint` and took
   checkpoint 4's review snapshot (18:12:41) within about a minute of the last write
   of the subagent that applied checkpoint 3's round-1 fixes (18:12:53). And the lead
   started checkpoint 4's round 2 at 18:40:38 while the triage-and-apply subagent made
   two last small edits to these notes, the last at 18:42:32. The reviewers were
   pointed at `ai_tmp/review/QS01-4/ticket-after-apply.md`, which that subagent
   refreshed at 18:42:42, so they may have read text two edits older. By the lead's
   account no machine ran during the third or the fourth. The files do not settle
   that for the fourth: the next machine log, `logs/old-token-after-restart.txt`, was
   last written at 18:44:19 and holds no start time.
   The session's own record, which is outside the worktree, puts the start of the
   command that wrote that log at 18:44:08, after both.
4. **The order of steps.** Claim 13's masks (step 9) were tested before step 6, because
   checkpoint 1's second round found that a guest with the worktree mounted read-write
   could rewrite the spike's own wrapper and profiles. Steps 6, 8, 10's control, 11 and
   12's timings ran before the sign-in stop, to keep the maintainer's part to one
   block; steps 7, 9's persistence and 10's provider call after it. Checkpoint 3 ran
   after step 6 had begun, not at the end of step 5.
5. **The agent did not open the token URL in the host browser** (step 6). Checkpoint 1
   found that the URL would stay in the browser's history; the maintainer chose to
   paste it from the clipboard into a private window, at step 7.
6. **Three machines, not one.** `qs01-a` (`--net`, the index digest) served steps 4 to
   6, 8 and 11 and the control boot of step 10. `qs01-s` (`--block-private`,
   `--deny-cidr 127.0.0.0/8`, the amd64 manifest digest, 12 mounts) is the one the
   maintainer signed in to, and served steps 7, 9, 10 and 12. Short-lived machines
   (`qs01-m`, `qs01-m2`, `qs01-n`, `qs01-v`, `qs01-p`) ran the mask, limit and volume
   tests. The image named in claim 3 is the index digest the first machine used; the
   one to carry forward is the manifest digest (claim 16).
7. **Studio's data was not put on a volume** (step 9), for want of a mount (claims 8
   and 16).
8. **A rename test damaged the worktree, and it was repaired.** The guest renamed
   `loadgen` and, a command later, renamed it back into a `loadgen/` that smolvm had
   recreated, leaving the tracked files under `loadgen/loadgen.qs01/` (claim 13). From
   the host, with the machine stopped: the empty `loadgen/target` and `loadgen` were
   removed with `rmdir`, and the tree moved back with `mv`. `git status --short` then
   listed only this ticket, and `loadgen/target` held its `CACHEDIR.TAG` and `debug`.
9. **`probe.err` is grouped, not quoted line by line**, with the maintainer's leave
   (section 9).
10. **Fixes were applied partly by the lead.** §14 has the triage subagents apply the
    valid fixes. They did so for the wrapper, the profiles and checkpoint 1's artefact,
    and for checkpoint 3's corrections to these notes. Where a fix needed a new boot or
    a new transcript, the lead ran it, since every machine command stayed in one
    session.
11. **A safety check of the agent's harness refused two commands**, each because a
    script held an `rm`: one that would have had the guest try to remove a file, so
    whether the guest can remove a `uchg` file was not tested (claim 13's workaround).
    And the lead once typed a direct call of the unwrapped smolvm launcher, which the
    harness refused before it ran.
12. **The reviewer's command.** `codex exec -m gpt-6.2-sol -s read-only -C "$PWD" -o <last file> "<brief>" < /dev/null >| <log> 2>&1 &`,
    AP01's form with the model changed, confirmed against `codex exec --help`
    (codex-cli 0.172.0). No fallback model was used.
13. **The latency of a UI action was not measured three times** (step 12). Start-up,
    an MCP `tools/list`, the MCP tool `read_project_summary` and the fetch of the UI's
    page were each timed three times (claim 11). The one UI action, a scan, was run
    once by the maintainer, who gave its duration from memory. The agent drove no
    browser (Departure 5), and the measurement will not be made in this ticket. The
    maintainer's verdict stands as quoted.
14. **`connect claude`'s files are recorded by name, and only the small ones by
    content** (step 11). The run wrote 21 files, and its saved output,
    `ai_tmp/qs01/logs/connect3-raw.txt`, is 3762 lines and is not committed. Claim 10
    lists every path and gives the content of the small JSON files. The run was also
    against a stand-in `claude` and did not complete, so the list is not the write
    set of a real connection.
15. **Step 14 changed more in these notes than it added, and grouped the probe's new
    lines** (step 14). Step 14 says to add the clean-up's output and change nothing
    else, and to list every line outside the worktree that the first probe lacked.
    The rerun gave 8633 such lines, and section 9 groups them by place and reason
    without the maintainer's leave for that list; the full list is
    `ai_tmp/qs01/probe-new-outside.txt`, not committed. Beside the output, the lead
    recorded the maintainer's two answers of 2026-11-25 in section 9, removed the
    line at the head of these notes that called them a draft in progress, and added
    this item. None of these changes a claim's outcome or a fact, so checkpoint 4
    was not run again. No reviewer saw step 14's output, the fixes of checkpoint 4's
    second round, or section 12. Step 14 also removed five files it does not name,
    the four token files and one saved page, as section 9 shows.

### 12. Verification

`ai_tmp/qs01/verify.sh`, the script under Verification with `base=` filled in from
section 1, ran as `sh ai_tmp/qs01/verify.sh` at
`de810414c9bb1b6a29a59b09a904dfaba8cb8bd8` on 2026-11-25 and exited 0. Under `set -eu`
it reaches its last line only when every command before it succeeded: `cmp` on the
two changed paths, `just check`, and the empty `git status --short`. Its output is
1127 lines, saved as `ai_tmp/qs01/logs/verify.txt`; `verify exit 0` is the line the
lead's command added after it. Excerpts, the first line and the last ones:

```text
de810414c9bb1b6a29a59b09a904dfaba8cb8bd8
bg-project-check run

==> just check-toolchain
```

```text
==> just check-clean
bg-project-check clean "$1"
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
verify exit 0
```

`just check` ran 21 recipes, each under a `==> just` line in the log: `check-toolchain`,
`lock-check`, `lint`, `fmt-check`, `toml-check`, `clippy`, `metrics`, `effects`,
`features`, `test-doc`, `coverage`, `doc`, `deny`, `deps-unused`, `check-docs`,
`check-agents`, `check-specs`, `analyse-specs`, `snapshots-check`, `test-doc-default`
and `check-clean`. The list `git diff --name-only` wrote, which `cmp` found equal to
the two paths of Files touched, `ai_tmp/qs01/changed.txt`:

```text
.scratch/quint-studio/QS01-studio-spike.md
.scratch/quint-studio/README.md
```

The check on the commit that records this section is step 14's last part, and goes to
the maintainer with the hand-back, since a commit cannot quote its own check.

## Open points

Points 1 to 8 and 14 are for the maintainer, before QS02 starts. Points 9 to 13, 15 and
16 were not settled by this spike.

1. **Ruling 2 and a secondary worktree's `.git`** (claim 13). The read-write mount lets
   the guest rewrite the pointer file, and a file cannot be masked. `chflags uchg` held
   against everything tried, on a throwaway file and never on `.git` itself, but
   removal by the guest was not tried, `git worktree move` and
   `repair` need it cleared, and a recipe that sets a flag on `.git` is a new kind of
   thing for this repository. The alternatives are to support Studio only from the
   primary checkout, where `.git` is a directory and can be masked, or to mount the
   worktree read-only with a few writable directories, which is not ruling 2.
2. **Masking the ancestors whole** (claim 13) makes `tools/`, `fuzz/` and `loadgen/`
   read-only to Studio, tracked files included.
3. **Twelve mounts** (claim 16) are all used by the tested layout, with the state
   directory sharing `ai_tmp`'s mask. QS02's state directory, wherever it goes, and any
   new ignored directory need a mount that does not exist. Fewer, coarser masks, or one
   parent directory for every project-managed ignored tree, would make room.
4. **The state root must be relative** (claim 2), so every recipe must run from the
   worktree root, and the state directory's path, relative to it, must leave socket
   paths within 103 bytes: at most 42 bytes for the root.
5. **Studio and git** (claims 7 and 8). Studio's scan needs to add objects. The tested
   answer is a wrapper around the guest's `git`; mounting the common directory
   read-write is the other, and gives the guest the hooks and configuration of every
   worktree. The common directory is the whole primary `.git`, readable by the guest.
6. **Port 7000 is AirPlay Receiver's** on this Mac (claim 5). QS04 should pick another
   host port, or its check must tell the two servers apart.
7. **Studio is not hermetic.** It downloads an executable into its data directory
   (`bin/quint-loops`), has an `upgrade` command, checks for updates, queues telemetry
   and carries a Sentry client. Pinning the AppImage's hash does not pin those.
8. **The guest packages are unpinned** (Facts for QS04), and the version string is not
   in the AppImage (claim 12). No snapshot timestamp and no derived image was
   install-tested, and the suite each of the final machine's 89 packages came from was
   not recorded; an install-tested snapshot or image digest is QS04's to produce.
9. Whether the mount limit depends on `--rosetta`; why the index digest gives an arm64
   guest; and what exactly overlaps when a second machine cannot be stopped (claim 16).
10. smolvm also ships as the PyPI package `smolmachines` (1.32.4 is installed in two
    other projects' environments on this host, sharing the default state directory).
    Claim 14 does not cover PyPI, and QS02 may want to say why the release tarball is
    preferred.
11. Sign-in by Google, the CLI `login`, what happens when the day-long access expires,
    and a call to the Anthropic provider were not exercised (claims 6 and 9).
12. Checkpoint 1's fallback, a removal script for `/private/tmp/ks-*`, was never used.
    Its second-round findings stand against it: an inventory by name cannot prove who
    made an empty alias directory, and a name with a newline defeats the comparison.
    A later ticket that ever needs the alias must not reuse it as written.
13. Linux hosts are untested (ruling 4).
14. **A read-only mask does not hide what it protects** (claim 13). In the tested
    layout the guest could read the state directory, with the registry pull token
    smolvm caches in it, and the rest of `ai_tmp/`. Hiding the state directory takes an
    empty-directory mask of its own. That is a thirteenth mount where twelve is the
    limit (Open point 3), and the read-write mount of the whole worktree leaves no
    place inside it that the guest does not see without one. An empty mask inside a
    read-only one was not tested.
15. Read from the source and not run: what a host without Rosetta does (claim 3);
    whether a named machine with no network has its image fetched on the host, and
    whether that fetch is why the index digest gives an arm64 guest (claim 16); that
    `machine update --no-net` and then `--net` drops the deny rules, and where an old
    template can come from after an upgrade (Facts for QS02); and how a Smolfile
    expands `$HOME` under the redirect (Facts for QS04). Not tested or not seen: the
    15 libraries and `ca-certificates` without `git` and `curl` on a new machine
    (claim 4); `connect claude` against a real `claude` (claim 10); the latency of a
    UI action (claim 11); and a Studio documentation page that names the MCP
    endpoint's options (claim 15). A later ticket that relies on any of these runs it
    first.
16. Left open by checkpoint 4's second round, the last one §14 allows. Not checked:
    whether the component map Studio answers with after a restart is the one it held
    before, since no answer was saved before the stop, and why the usage reading
    taken after three restarts lists nothing under `by project` (claim 8); and the
    browser's own cookie after a restart, since only a token read for the test was
    replayed (claim 5 and section 9). Read from the source and not run: that an
    ephemeral `machine run`
    with a deny CIDR has its image fetched on the host, with the platform it asks for
    (Facts for QS04); and that `machine ls --verbose` counts a staged mount without
    printing it, while `machine status --json` carries `shared_folders` (claim 16).
    Not written: a git wrapper that compares the common directory with the exact
    mounted path, which is QS04's to write and test (Facts for QS04). A later ticket
    that relies on any of these runs it first.
