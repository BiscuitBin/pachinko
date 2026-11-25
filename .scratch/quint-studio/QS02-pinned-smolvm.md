---
id: QS02
title: "Decision 0022, a pinned smolvm, and a project-managed machine that boots"
status: todo
depends_on: [QS01]
parallel_with: [QS03]
branch: ticket/qs02-pinned-smolvm
estimated_size: M
---

# QS02: Decision 0022, a pinned smolvm, and a project-managed machine that boots

## Context

QS01, `.scratch/quint-studio/QS01-studio-spike.md`, ran Quint Studio's `serve` inside a
smolvm guest by hand and recorded a go or no-go. This ticket runs only on go. It makes
smolvm a tool this repository holds the way AP02 holds Apalache: a version and a SHA-256
in a pin file, an installer into a gitignored directory with a stamp inside the tree it
certifies, an offline check of the pin file in gate 2, and a recipe that runs only the
stamped install. It also writes decision 0022, which reverses decision 0021's sentence
that Quint Studio "stays outside the repository".

This design assumes that neither `pixi.lock` nor `tools.txt` (decision 0011) can hold
smolvm, which QS01 claim 14 checks: no conda-forge package, and no crates.io crate that
cargo-binstall could install. If claim 14 found either, this ticket stops at step 1 and
goes back to the maintainer: that route would change its design and its Files touched
(`pyproject.toml` and `pixi.lock`, or `tools.txt`), so the ticket is rewritten before
it runs (Open points). Its
release tarballs ship with `checksums.sha256` (QS01 claim 1). It runs guests on
Hypervisor.framework through an entitlement on its signed binary, so the installer
never re-signs it, never strips its extended attributes, and checks the entitlement
after it unpacks.

The maintainer's rulings of 2026-11-24, which QS01's Context lists in full, bind this
ticket: Studio is pinned by its observed SHA-256 (ruling 1); the worktree is mounted
read-write, with open egress and host loopback and private ranges blocked (ruling 2);
agents are wired to Studio's MCP endpoint in QS05 (ruling 3); and the only supported host
is macOS on arm64, with Linux recorded as untested (ruling 4).

Read these:

- `.scratch/quint-studio/QS01-studio-spike.md`, its hand-back notes, all of them. Every
  name, path, flag and hash this ticket uses comes from there, never from this text.
- `.scratch/apalache/AP02-checksummed-apalache.md`, "Design proposed in this ticket" and
  "Steps", for the installer, the stamp and the gate's red tests this ticket copies.
- `.scratch/apalache/AP01-java-and-verify.md`, "Adversarial review".
- `docs/decisions/0021-project-managed-quint-cli.md` and
  `docs/decisions/0005-project-managed-allium-cli.md`.
- `Justfile`: `initialize`, `install-apalache`, `lock-check` and `quint`.
  `scripts/initialize.sh`, `scripts/install-apalache.sh`,
  `scripts/check-apalache-pin.sh`.
- `tickets/CONVENTIONS.md` §11, §13 and §14.

## Goal

`just install-smolvm` installs smolvm v1.37.4 into `.tools/smolvm/` from the release
tarball whose SHA-256 `tools/smolvm/smolvm.txt` records, checks its entitlement, and
writes a stamp. A rerun over the stamped install skips and reaches no network.
`just smolvm <args>` runs only that stamped install, by path, with its machine state kept
inside the worktree if QS01 claim 2 holds. Gate 2, `just lock-check`, fails offline on a
malformed pin file. Decision 0022 records smolvm as a tool, the Studio pin policy, the
guest's access and the supported host, and decision 0021's Studio sentences point at it.

The demo: a machine booted by `just smolvm` from the guest image QS01 pinned mounts the
worktree and answers on a port published to host loopback.

## Design proposed in this ticket

These are proposals. Step 1 amends them where QS01's hand-back disagrees, and the
maintainer rules on the result at step 4, with checkpoint 1's findings in hand. After
that the builder does not reopen them.

- **One pin file, `tools/smolvm/smolvm.txt`.** Comment lines start with `#`. Three
  `key=value` lines follow: `version=` (`1.37.4`), `asset=` the darwin-arm64 tarball's
  name as the release lists it, and `sha256=` its hash, lowercase hex, equal to the line
  for that asset in the release's `checksums.sha256`. The scripts read it with `sed` and
  never source it.
- **`scripts/check-smolvm-pin.sh`, in `lock-check`.** Offline and read-only, reading only
  committed files. It fails unless the file has exactly the three keys, `version` is
  `x.y.z`, `asset` names `version` and ends in the archive suffix QS01 recorded, and
  `sha256` is 64 lowercase hex digits. Each failure exits 1 and prints a line of the form
  `tools/smolvm/smolvm.txt: <key>: <reason>`, which Verification matches.
- **`scripts/install-smolvm.sh`, run by `just install-smolvm`.** In order:
  1. It refuses any host but macOS on arm64 (`uname -s` is `Darwin`, `uname -m` is
     `arm64`), naming ruling 4 and decision 0022.
  2. It fails if `scripts/check-smolvm-pin.sh` would.
  3. It refuses a symlink, or anything but a directory, at `.tools` or `.tools/smolvm`,
     naming the recovery (remove the symlink itself, not its target), as
     `scripts/install-apalache.sh` does for its own paths. Without that a symlinked
     `.tools` would carry the temporary directory and the install outside the worktree.
  4. It skips, and says so, only if `.tools/smolvm/.pachinko-stamp` exists, its
     `archive=` is the pinned hash, its `tree=` equals a digest of the tree recomputed
     now (AP02's digest: the SHA-256 of the sorted `<sha256>  <path>` lines of every file
     but the stamp), and the binary is executable.
  5. It downloads the tarball with `curl -fsSL --proto '=https'` into a new temporary
     directory under `.tools/` (from `mktemp -d`), checks its SHA-256 with
     `shasum -a 256`, and on a mismatch prints both hashes, removes the temporary
     directory and exits non-zero.
  6. It unpacks into the temporary directory, preserving the archive's file modes and
     extended attributes, with `TAR_OPTIONS`, `TAR_READER_OPTIONS`, `TAR_WRITER_OPTIONS`
     and `TAPE` unset, as AP02's installer does. Before extracting it checks every member
     name in the archive's listing (no absolute path, no `..` component); after, it
     requires the extracted set of files to equal that listing, and to hold every file
     QS01 recorded the binary needs beside it. It checks with `codesign --verify --strict`
     and `codesign -d --entitlements -` that the binary is signed and holds
     `com.apple.security.hypervisor`, writes the stamp, and only then replaces
     `.tools/smolvm/` with it. It never calls `codesign --sign`,
     `codesign --remove-signature` or `xattr`.
  7. Every `rm -rf` names its path through `"${var:?}"`, and a trap removes the
     temporary directory on every exit.
- **`--check` mode.** `scripts/install-smolvm.sh --check` is offline and read-only: it
  runs checks 1 to 3 and step 4's skip test, and exits 0 only when that test would skip.
  Otherwise it exits 1 and prints a message naming `just install-smolvm`.
- **`just smolvm <args>`.** Its first line runs `scripts/install-smolvm.sh --check`,
  failing and naming `just install-smolvm` when it fails; its second runs the stamped
  binary by path, never a `smolvm` from `PATH`. Neither line is quieted with `@`, so
  `just` echoes each before it runs it, and Verification uses the second line's echo as
  the sentinel that smolvm was reached. `scripts/install-smolvm.sh` never runs the
  binary in `--check` mode. If QS01 claim 2 holds, the recipe sets the variable or
  option QS01 named so that machine state, the image cache and volumes live in
  `.smolvm/`, which Git ignores, refuses a symlink, or anything but a directory, at
  `.smolvm`, and refuses a value the caller set that points elsewhere, each exiting 1 in
  the first line, before smolvm starts. If claim 2 fails, the maintainer rules at step 4
  between state at the home-directory path QS01 recorded, named in decision 0022 and the
  pages, and no-go.
- **`just initialize`** runs `just install-smolvm` after `just install-apalache`, on
  macOS arm64 only. On any other host it prints that smolvm and Quint Studio are skipped
  on an untested host, naming decision 0022, and goes on. The alternative, leaving
  smolvm out of `initialize` so that only a worktree that wants Studio downloads it, is
  in Open points for the ruling.
- **No Renovate manager reads `tools/smolvm/smolvm.txt`.** smolvm is moved by hand, as
  Apalache is, and `docs/how-to/maintain-dependencies.md` says how.
- **Decision 0022, `docs/decisions/0022-quint-studio-machine.md`** (0022 is the next free
  number; confirm it on the base at step 1). It records:
  - smolvm as a tool, pinned and installed as above, and why neither pixi nor
    `tools.txt` holds it, from QS01 claim 14's outcome;
  - the Studio pin policy of ruling 1, which QS03 implements, with its limit stated:
    it detects drift and does not make installs reproducible;
  - the guest's access under ruling 2: the worktree read-write at its own path, the
    common git directory as QS01 claim 7 found, open egress, host loopback and private
    ranges blocked. It states the consequence: the guest can write any file in the
    worktree, the `Justfile` and the scripts among them, which the host later runs, so a
    compromised guest or Studio build can reach the host through the next recipe. For
    tracked files the control is the maintainer reading the diff. Gitignored paths are
    outside every diff, and the host runs or loads some of them: `.tools/`, `.pixi/`,
    `.quint/`, `tools/quint/node_modules/`, the `target/` directories, `.claude/`,
    `.codex/`, `ai_tmp/` and the machine state, the list QS01 claim 13 tested; QS04
    rechecks it against `.gitignore`. The decision says how QS04 masks each one, as
    QS01 claim 13 found, or, if masking failed, records the conflict between the
    read-write ruling and state kept in the worktree, and the maintainer's ruling on it;
  - the supported host under ruling 4, and that CI never boots a machine, since no
    runner offers the hypervisor;
  - where the machine's state lives, from QS01 claim 2.

  Decision 0021's Context and Decision paragraphs on Studio are amended to point at
  0022, and the record is registered in `docs/manifest.yml` and
  `docs/decisions/README.md`.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 1 | The installed binary keeps its signature and entitlement after the installer's unpack | step 6, `codesign` read-only, and a boot |
| 2 | `just install-smolvm` over a stamped install reaches no network | step 7, under §11's denial with its negative controls |
| 3 | Under `just smolvm`, nothing is written outside the worktree, beyond what decision 0022 names, in any location QS01 step 3's probe can read. Locations it cannot read are listed as unprobed, and the claim stays open for them | step 7, QS01 step 3's probe with its controls |

## Non-goals

- No Studio. QS03 pins it, QS04 runs it.
- No committed Smolfile. QS04 writes it; this ticket's demo passes flags.
- No CI job that installs smolvm or boots a machine.
- No Linux host support beyond the skip in `initialize` and the refusal in the
  installer.
- No `CHANGELOG.md` entry. It records the crates' changes.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review" (T37 is
still todo). One delta overrides them: the reviewer is Codex Sol, `-m gpt-6.2-sol`, in
place of the `-m gpt-6-astra` that AP01's command and §14 print, by the maintainer's
choice of 2026-11-25. AP01's fallback to a fresh-context `claude-opus-5-5` subagent
still applies, and the hand-back notes record the model that ran. `<id>` is `QS02`.
There are two checkpoints, at steps 3 and 9.

## Files touched

Each page row covers every sentence on that page that lists what `just initialize`,
gate 2 or the network recipes do, or says Studio cannot be held by the repository.

| Path | Class | Content |
| --- | --- | --- |
| `tools/smolvm/smolvm.txt` | new | the pin file, with a comment naming decision 0022 and the bump how-to |
| `scripts/install-smolvm.sh` | new | the installer, POSIX `sh`, with `--check` |
| `scripts/check-smolvm-pin.sh` | new | the gate's check, POSIX `sh` |
| `docs/decisions/0022-quint-studio-machine.md` | new | the decision record |
| `Justfile` | shared, own lines | `install-smolvm` and `smolvm`, each with a comment; a line in `lock-check` and its comment |
| `scripts/initialize.sh` | shared, own lines | only if the step 4 ruling keeps smolvm in `just initialize`: `just install-smolvm` after `just install-apalache`, on macOS arm64 only, with the skip message |
| `.gitignore` | shared, own lines | `/.smolvm/`, with a comment, if QS01 claim 2 holds; `.tools/` already covers the install |
| `AGENTS.md` | shared, own lines | `just install-smolvm` and `just smolvm` in the list of recipes that reach the network |
| `docs/decisions/0021-project-managed-quint-cli.md` | shared, own lines | the Studio sentences in Context and Decision point at 0022 |
| `docs/decisions/README.md` | shared, own lines | 0022's row |
| `docs/manifest.yml` | shared, own lines | 0022's entry |
| `docs/how-to/develop-locally.md` | shared, own lines | what `just initialize` installs, or, if the ruling leaves smolvm out, that `just install-smolvm` is run by hand; "Optional: Quint Studio" points at 0022 until QS04's page lands |
| `docs/tutorials/first-change.md` | shared, own lines | only if the step 4 ruling keeps smolvm in `just initialize`: its list of what `just initialize` installs gains smolvm, on macOS arm64 only |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | a subsection on moving the smolvm pin; gate 2's description |
| `docs/reference/commands.md` | shared, own lines | `install-smolvm` and `smolvm` rows, the network list, and the `initialize` and `lock-check` rows |
| `docs/reference/configuration.md` | shared, own lines | a row for `tools/smolvm/smolvm.txt` and the pins table |
| `docs/reference/quality-gates.md` | shared, own lines | gate 2's row and the recipes that reach the network |
| `docs/explanation/security-model.md` | shared, own lines | smolvm checked by its pin; the guest's access from decision 0022 |
| `docs/project/repository-map.md` | shared, own lines | `tools/smolvm/` and the two scripts |
| `.scratch/quint-studio/QS02-pinned-smolvm.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS02's Status cell |

## Steps

1. **Worktree, baseline and QS01's facts.** Create the worktree on
   `ticket/qs02-pinned-smolvm` from a base that holds QS01's hand-back, and record the
   base and its full commit id. Run `just initialize` (network, authorised) and
   `just check` on the untouched worktree, and quote its last line. Confirm that 0022 is
   the next free decision number on the base. Read QS01's hand-back notes. Write
   `ai_tmp/qs02/amendments.md`: every point of "Design proposed" that a QS01 claim
   contradicts or leaves open, and the change it needs. If QS01 was no-go, stop here.
2. **Read before writing** (network, authorised). Download the tarball and
   `checksums.sha256` into a new, empty `ai_tmp/qs02/download/`, compute the SHA-256,
   and compare it with the published line and with QS01's record. Never copy a hash from
   a page or a chat. Write `ai_tmp/qs02/plan.md`: the pin file's content, each script's
   outline against "Design proposed" as amended, decision 0022's outline, and the edit to
   each page in Files touched.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qs02/plan.md` and `ai_tmp/qs02/amendments.md`, before any tracked file
   changes). Questions:
   - Can the installer skip over a tree it did not unpack and check, or over one whose
     binary lost its signature?
   - Can any step of the installer re-sign the binary or strip an attribute, directly or
     through a tool it calls?
   - Can `just smolvm` run a binary other than the stamped one, or write machine state
     outside the place decision 0022 names?
   - Does each `rm -rf` fail closed when its variable is empty?
   - Can a symlinked `.tools`, `.tools/smolvm` or `.smolvm` carry a write outside the
     worktree, or an inherited `tar` option leave a tree that is stamped but incomplete?
   - Does decision 0022 state the read-write mount's consequence honestly?
4. **Red: the gate.** **Stop** first for the maintainer's ruling on "Design proposed" as
   amended, with the checkpoint's findings in hand, and record it. Then add the pin file,
   `scripts/check-smolvm-pin.sh` and the `lock-check` line. Save the pin file to
   `ai_tmp/qs02/smolvm.txt.saved`. Show `just lock-check` passing. Then, each on an edit
   restored from the saved copy afterwards, show it failing and quote each message:
   `version=1.37` (two parts); the `sha256=` line removed; `sha256=` given 63 digits; an
   `asset=` naming another version; a fourth key added. After the last restore,
   `cmp tools/smolvm/smolvm.txt ai_tmp/qs02/smolvm.txt.saved` exits 0.
5. **The installer.** Add `scripts/install-smolvm.sh`, `just install-smolvm`, and, if
   the ruling keeps it, the line in `scripts/initialize.sh`. Then, quoting each run and
   its exit status and
   restoring each edit afterwards:
   - `just install-smolvm` (network, authorised) installs and writes the stamp;
   - a second run skips and says so;
   - one byte appended to a file under `.tools/smolvm/` that is not the stamp: the
     installer replaces the tree (network, authorised);
   - the stamp removed: the installer replaces the tree (network, authorised);
   - `chmod a-x` on the binary: the installer replaces the tree (network, authorised);
   - `sha256=` changed by one digit and `.tools/smolvm/` removed: the installer
     (network, authorised) fails, prints both hashes, and leaves no `.tools/smolvm/` and
     no temporary directory;
   - `TAR_OPTIONS` set to exclude one file QS01 recorded the binary needs beside it, with
     `.tools/smolvm/` removed first: the installer (network, authorised) still installs
     the full tree, and `ls` shows that file present;
   - `.tools/smolvm` replaced by a symlink to a new, empty directory under
     `ai_tmp/qs02/`, and separately `.tools` moved aside and replaced by a symlink to a
     copy of it under `ai_tmp/qs02/`: the installer refuses each, exiting 1 and naming
     the recovery, and the symlink's target is unchanged (`ls -la` before and after);
   - the host check, shown by running the script with `uname` shadowed by a stub that
     prints `Linux` and `x86_64` from a directory first on `PATH` under `ai_tmp/qs02/`:
     it refuses and names decision 0022;
   - the pin restored and `just install-smolvm` run once more (network, authorised).
6. **The recipe and the demo.** Add `just smolvm`. Show, quoting each:
   - with the stamp restored, `just smolvm --version` prints `just`'s echo of the
     recipe's second line and the exact line QS01 recorded; with the stamp moved to
     `ai_tmp/qs02/stamp.saved`, it exits 1, naming `just install-smolvm`, with neither
     line and with `just`'s failure message naming the recipe's first line, so smolvm
     was never reached; with it restored, `cmp` exits 0. `grep` shows that
     `scripts/install-smolvm.sh` holds no command that runs the installed binary;
   - if QS01 claim 2 holds, `.smolvm` replaced by a symlink to a new, empty directory
     under `ai_tmp/qs02/`: `just smolvm --version` exits 1, naming the recovery, prints
     no version line, and the target stays empty;
   - a `smolvm` stub first on `PATH` under `ai_tmp/qs02/` is not the one run;
   - `codesign --verify --strict` and `codesign -d --entitlements -` on the installed
     binary, read-only (claim 1);
   - the demo (network, authorised: the image pull): `just smolvm` boots the image QS01
     pinned by digest, with `--rosetta`, the worktree mounted at its own path, a listener
     QS01's facts name bound to `0.0.0.0` in the guest and published on
     `127.0.0.1:<port>`. From the host, `curl -fsS http://127.0.0.1:<port>/` answers, and
     the guest lists a file the host created in the worktree a moment before. Then stop
     and delete the machine.
7. **No network, no home directory.** Run `just install-smolvm` under §11's macOS
   denial, `sandbox-exec -p '(version 1)(allow default)(deny network*)'`, with both of
   §11's `curl` negative controls failing inside the same denial, and quote all three exit
   codes (claim 2). Rerun step 6's demo with QS01 step 3's probe, its marker under
   `ai_tmp/qs02/`, with its controls and its unreadable directories recorded, and show
   that nothing outside the worktree changed beyond what decision 0022 names in any
   location the probe can read, listing each location it could not read as unprobed,
   where claim 3 stays open (claim 3).
8. **Words.** Write decision 0022 and edit each page in Files touched. Run
   `just check-docs` and `just check-agents`.
9. **Adversarial review** (checkpoint 2: conformance and breakage, over the diff with the
   pages included, before the final commit). Questions:
   - Does `lock-check` stay offline and read-only, and does `just check` stay free of the
     network and of smolvm?
   - Could Verification pass while `just smolvm` ran an unstamped binary?
   - What does `just initialize` do on Linux, and in a worktree where the download fails?
   - Does every page agree on where the pin lives, what checks it, where state lives, and
     what the guest can reach and write? Does any page outside Files touched still say
     Studio cannot be held by the repository?
10. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed by
    the full commit id. Set `status: done` here and in `.scratch/quint-studio/README.md`,
    write the hand-back notes, and commit the record. On the record commit, run
    `git diff --name-only HEAD~1 HEAD`, which must list only this ticket and the
    README, then `just check` and `git status --short`, and report their output to the
    maintainer with the hand-back, headed by that commit, since a commit cannot quote
    its own check. If any fails, fix the cause in a new commit and run all three again.
    **Stop** before pushing or opening a pull request: both need authorisation
    (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `tools/smolvm/smolvm.txt` names `version=1.37.4`, the asset QS01 recorded and the hash
  step 2 computed, equal to the release's own line; each mutation of step 4 makes
  `just lock-check` fail, each quoted.
- `just install-smolvm` installs, skips a stamped install, replaces a changed, unstamped
  or non-executable tree, refuses a wrong hash leaving nothing behind, installs the full
  tree despite an inherited `TAR_OPTIONS`, refuses a symlinked `.tools` or
  `.tools/smolvm` leaving its target unchanged, and refuses another host, each quoted.
- The installed binary is signed and holds the Hypervisor entitlement, quoted.
- `just smolvm` refuses without the stamp and, if QS01 claim 2 holds, on a symlinked
  `.smolvm`, each with exit 1, its diagnostic and no start of smolvm, never runs a
  `smolvm` from `PATH`, and keeps its state where decision 0022 says.
- The demo boots, mounts the worktree and answers on host loopback, quoted.
- `just install-smolvm` over a stamped install exits 0 under the network denial, with
  both negative controls failing.
- `AGENTS.md` lists `just install-smolvm` and `just smolvm` among the recipes that reach
  the network, and `just check-agents` passes.
- Decision 0022 exists, is registered, and records the four rulings and the read-write
  mount's consequence; decision 0021 points at it.
- Each claim has its outcome recorded.
- Both checkpoints are recorded as §14's Record asks, with AP01's deltas.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record
  commit, as step 10 reports.

## Verification

Run from a file as one `sh` script; `set -eu` stops it at the first command that fails.
No pipeline carries a result: each command writes to a file, its exit status is checked,
and the file is matched exactly.

```sh
set -eu
git rev-parse HEAD
version_line='<the exact smolvm --version line QS01 recorded>'
exec_line='<the line just echoes for the recipe running the stamped binary, quoted at step 6>'
guard_failed='<the failure line just prints, naming the first line of the recipe, quoted at step 6>'
pin_file=tools/smolvm/smolvm.txt
stamp=.tools/smolvm/.pachinko-stamp
out=ai_tmp/qs02/verify
mkdir -p "$out"
cp "$pin_file" "$out/smolvm.txt"
restore() {
    cp "$out/smolvm.txt" "$pin_file"
    if [ -e "$out/stamp" ]; then mv "$out/stamp" "$stamp"; fi
}
trap restore EXIT

# Green first: offline check, then the stamped install runs and prints its line.
just lock-check
sh scripts/install-smolvm.sh --check
just install-smolvm >| "$out/install.log" 2>&1
pin=$(sed -n 's/^sha256=//p' "$pin_file")
test -n "$pin"
grep -Fx "archive=$pin" "$stamp"
just smolvm --version >| "$out/version.log" 2>&1
grep -Fx "$exec_line" "$out/version.log"
grep -Fx "$version_line" "$out/version.log"

# Red: a malformed sha256= fails the gate with the pin check's own message.
sed 's/^sha256=.*/sha256=0/' "$out/smolvm.txt" >| "$pin_file"
rc=0; just lock-check >| "$out/lock-check.log" 2>&1 || rc=$?
cp "$out/smolvm.txt" "$pin_file"
test "$rc" -eq 1
grep -F 'tools/smolvm/smolvm.txt: sha256:' "$out/lock-check.log"
git diff --exit-code "$pin_file"

# Red: without the stamp, just smolvm refuses before smolvm starts.
mv "$stamp" "$out/stamp"
rc=0; just smolvm --version >| "$out/refuse.log" 2>&1 || rc=$?
mv "$out/stamp" "$stamp"
test "$rc" -eq 1
grep -F 'just install-smolvm' "$out/refuse.log"
grep -Fx "$guard_failed" "$out/refuse.log"
if grep -Fx "$exec_line" "$out/refuse.log"; then exit 1; else test "$?" -eq 1; fi
if grep -Fx "$version_line" "$out/refuse.log"; then exit 1; else test "$?" -eq 1; fi
grep -Fx "archive=$pin" "$stamp"

just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

`version_line` is filled in from QS01's hand-back, and `exec_line` and `guard_failed`
from step 6's quotes, before the script runs. The green run of `just smolvm --version`
is the positive control for the refusal's silent probes: `just`'s echo of the line that
runs the binary, and the version line, each matched whole, appear when smolvm is reached
and must both be absent when the guard refuses, while `just` names the first line as the
one that failed. The binary is signed and cannot be wrapped, so the sentinel is the
recipe's own echo rather than a shim; step 6 shows the guard script never runs it. The
EXIT trap restores the pin file and the stamp if any line fails between a mutation and
its restore. Steps 4 to 7 hold the other mutations, the demo and the network denial,
whose output is quoted there.

## Open points

- Whether `just initialize` installs smolvm at all, or leaves it to a worktree that runs
  Studio. The design proposes it does, on macOS arm64; the maintainer rules at step 4.
- If QS01 claim 14 found smolvm on conda-forge or crates.io, whether that route replaces
  this installer. This ticket does not cover it: its design and Files touched assume
  neither route, so the maintainer rules on the route and the ticket is rewritten
  before step 2.

## Hand-back notes

Empty until the ticket runs. Sections, in order: Base and branch; Baseline; QS01's facts
and the amendments; Ruling; Claims; What landed; The gate and the installer, red and
green; Checkpoint 1 record; Checkpoint 2 record; Departures; Verification; Open points.
