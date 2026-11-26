---
id: QS02
title: "Decision 0022, a pinned smolvm, and a project-managed machine that boots"
status: done
depends_on: [QS01]
parallel_with: [QS03]
branch: ticket/qs02-pinned-smolvm
estimated_size: L
---

# QS02: Decision 0022, a pinned smolvm, and a project-managed machine that boots

## Context

QS01, `.scratch/quint-studio/QS01-studio-spike.md`, ran Quint Studio's `serve` inside a
smolvm guest by hand on 2026-11-25. With the rulings the maintainer gave after its
hand-back, its outcome is go, and the README of these tickets,
`.scratch/quint-studio/README.md`, records that. Two of those rulings were never run by
QS01, so this ticket proves them by hand before it builds on them (step 3), and a proof
that fails stops the series there for the maintainer. This ticket makes
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

The maintainer's rulings, which the README of these tickets states in full, bind this
ticket. Rulings 1 to 4 were taken on 2026-11-24. On 2026-11-25, after QS01's hand-back,
ruling 2 was amended and rulings 5 to 10 were taken. Studio is pinned by its observed
SHA-256 (ruling 1). The worktree is mounted read-only, with a few writable
subdirectories, and the guest has open egress with host loopback and private ranges
blocked (ruling 2, as amended); mounting it at its own path is how QS01 ran it and
what this ticket proposes, not part of the ruling. Agents are wired to Studio's MCP endpoint in QS05
(ruling 3). The only supported host is macOS on arm64, with Linux recorded as untested
(ruling 4). smolvm's machine state lives outside the worktree, in a durable directory of
its own for each worktree, and smolvm is given a short symbolic link to it in `/tmp`,
because its socket paths must stay short (ruling 5). This ticket builds ruling 5 and
shows that the mounts of ruling 2 work, neither of which QS01 ran. Decision 0022 also
records rulings 6 to 10: hiding (6), git (7), the host port (8), what the pin does not
cover (9) and the guest's packages (10), each with the ticket that implements it.

Two terms are used the same way here, in QS04 and in decision 0022. **The state
directory** is the durable directory outside the worktree, one for each worktree, that
holds smolvm's machine state. **The state link** is the short symbolic link in `/tmp`
that smolvm is given as its state root.

Read these:

- `.scratch/quint-studio/README.md`, "The maintainer's rulings" and "Before QS01
  starts". The rulings are cited from there by number.
- `.scratch/quint-studio/QS01-studio-spike.md`, its hand-back notes, all of them, and
  its "Rulings after the hand-back" section. Every name, path, flag and hash this
  ticket uses comes from there, never from this text. This ticket leans most on claims
  2, 13, 14 and 16, on "Facts for QS02", and on section 9, "What was written outside
  the worktree, and its removal", for the wrapper, the profile and its controls.
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
`just smolvm <args>` runs only that stamped install, by path, in a controlled
environment, with its machine state kept in the state directory outside the worktree
that ruling 5 names, reached through the state link. `just smolvm-purge` removes that
state, for this worktree only. Gate 2, `just lock-check`, fails offline on a malformed
pin file. Decision 0022 records smolvm as a tool, the Studio pin policy, the guest's
access, where the machine's state lives and what removes it, and the supported host,
and decision 0021's Studio sentences point at it.

The demo: a machine booted by `just smolvm` from the guest image QS01 pinned mounts the
worktree read-only at its own path, with one writable subdirectory inside it, and
answers on a port published to host loopback.

## Design proposed in this ticket

These are proposals. Step 1 amends them where QS01's hand-back disagrees, and the
maintainer rules on the result at step 5, with step 3's proofs and checkpoint 1's
findings in hand. After that the builder does not reopen them. The names given here for
things outside the worktree (the state directory's path, the state link's name and
`just smolvm-purge`) are proposals like the rest, and that ruling can change them.

Where a bullet says what must hold and names a claim or a "Facts for" section of QS01's
hand-back, step 1 reads the particulars there and writes them into
`ai_tmp/qs02/amendments.md`. This text repeats none of them.

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
     but the stamp, as "The stamp and a first boot" below amends it), and the binary is
     executable.
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
- **The stamp and a first boot.** Facts for QS02 says what smolvm writes beside its own
  binary on first use, which changes the tree the stamp certifies. Step 1 amends either
  the stamp's digest or the place of those files, so that the first boot after an
  install leaves `--check` passing. Whatever the amendment, the digest has one
  definition: the installer when it writes the stamp, its skip test and `--check` all
  compute it over the same files. Step 7's demo starts from a new install and shows
  that a first boot leaves `--check` passing.
- **`just smolvm <args>`.** One recipe line, which runs **the launcher**: one
  invocation that checks, then runs the stamped binary by path, never a `smolvm` from
  `PATH`, and ends when smolvm does, with smolvm's exit status. The checks are **the
  guard**. Every refusal below happens in it, exits 1 and names the recovery, before
  smolvm starts. The guard and smolvm's run are one invocation and not two recipe
  lines, because `just` gives each line a shell of its own, and a lock taken by one
  line is gone before the next starts. The installer never runs the binary, in
  `--check` mode or any other. Facts for QS02 says which file of the release is the
  one to run: step 1 amends "the stamped binary", and the file each `codesign` check
  reads, wherever that changes them in this text. The launcher, in order:
  1. It runs `scripts/install-smolvm.sh --check`, failing and naming
     `just install-smolvm` when it fails.
  2. It refuses a value the caller set, in the environment or in `<args>`, for any of
     smolvm's own variables and options that Facts for QS02 lists as moving its state or
     overriding what it loads. This comes before the environment is cleared, so that
     clearing cannot hide the caller's mistake.
  3. It takes the lock, which the bullet "One lock" below describes, and holds it until
     smolvm's run has ended.
  4. Under the lock it makes, or checks, the state directory, its marker and the state
     link, as the next three bullets say.
  5. It prints one fixed line on standard error, the sentinel that Verification
     matches, and its next command runs smolvm. Nothing that can fail lies between the
     two, so the sentinel is printed when smolvm is reached and is absent from every
     refusal.

  The launcher runs smolvm in the controlled environment QS01's wrapper used, which
  Facts for QS02 and section 9 of QS01's hand-back name: a cleared environment that
  holds only what they list, the registry configuration pinned to an empty one, a
  temporary directory under the launcher's control, and the state link passed as the
  state root the way claim 2 and Facts for QS02 name, as it is written and never
  resolved. The proof of step 3 under "Steps" and the launcher use the same
  environment. The launcher writes nothing of its own on standard output: its sentinel
  and its refusals go to standard error, so a caller that reads smolvm's standard
  output reads smolvm's alone. Later tickets call it too: QS04's script runs every
  smolvm command through it.
- **The state directory** (ruling 5) is
  `~/Library/Caches/pachinko/smolvm/<worktree>-<hash>/`, mode 0700. `<worktree>` and
  `<hash>` are formed as QS04 forms its machine name: the worktree's directory name
  lowercased, with every character outside `a-z`, `0-9` and `-` replaced by `-`, and
  the first eight hex digits of the SHA-256 of the worktree's absolute path. The guard
  checks before it creates. It goes down from the home directory, and each parent of
  the state directory below the home directory is checked before anything is made
  below it: it must be a real directory, owned by the user and writable by no one
  else. A parent that is absent is made, writable by no one else, and then checked in
  the same way. Only below parents that passed does the guard create the state
  directory, with its marker, if it is absent. Otherwise it refuses a symlink, or
  anything but a directory owned by the user, at the state directory.
- **The marker** is a regular file in the state directory, written when the directory
  is made, at a place where QS01 claim 16 shows smolvm leaves a host file alone. It
  records the worktree's physical absolute path and a random id, and the worktree
  keeps the same id in an ignored file of its own. Reuse needs both to match. A
  missing, unreadable or symlinked marker, or a mismatch of the path or of the id, is a
  refusal that names the recovery. So two worktrees whose hashes collide never share
  state, a worktree made again at an old path does not silently adopt the old state,
  and an orphan can be traced to its worktree after the worktree is gone.
- **The state link** is `/tmp/pachinko-smolvm-<hash>`, a symbolic link to the state
  directory, with the same `<hash>`. Facts for QS02 gives the limit on a socket path
  as it is written, and so on the root: the guard checks the link's name against that
  limit and refuses a longer one. The guard creates the link if it is absent, with a
  command that fails if the name exists and can neither replace an existing entry nor
  descend into one. It then checks the link without following it, and refuses unless
  it is a symbolic link, owned by the user, whose target is exactly the state
  directory. `/tmp` is shared and a restart empties it. So a name someone else made, a
  directory, or a link elsewhere is a refusal and is never followed, and a link that
  is absent after a restart is made again.
- **One lock** for each worktree's state, outside the state directory, serialises
  making the state, every machine operation and the purge. One invocation holds it
  from before the state is checked until what it protects is done: the launcher until
  smolvm's run has ended, the purge until its inventory and its removal have ended.
  Whatever holds it checks the state directory, the marker and the state link under
  it before it acts on them, so nothing can remove or swap the state between its
  check and its use. The lock is released when its holder ends, also when that fails
  or is interrupted, and a machine that goes on running after its command has
  returned does not keep it. The lock's own entry is stable: neither the purge nor
  the orphan's entry point removes it, since two callers that found and missed a
  removed entry would hold two locks.
- **`just smolvm-purge`** removes this worktree's state and nothing else, in one
  invocation. It runs `scripts/install-smolvm.sh --check` first, since its inventory
  needs the stamped install. It takes the lock and, under it, validates the state
  directory, the marker and the state link as the guard does, making an absent state
  link again as the guard does. Still under the lock it takes the inventory: the
  stamped smolvm's own listing of the machines in that state, run in the same
  controlled environment. It refuses, exiting 1 and removing nothing, if the listing
  fails for any reason or lists any machine, and it names each machine left and how
  to delete it. Otherwise it removes the state link and then the state directory,
  each through `"${var:?}"`, and prints what it removed. Everything smolvm keeps there
  goes with it. The lock's entry stays. Whether a parent that the purge leaves empty
  stays too is in Open points.
- **An orphan** is a state directory whose marker matches no worktree: the path the
  marker records is gone, or what is there now does not hold the marker's id. It
  cannot be purged from its own worktree. It is removed through a documented entry
  point, run from a surviving checkout and given the orphan's state directory. The
  entry point holds to the purge's rules: the same checks of ownership on the
  directory and its parents, that state's own lock, and an inventory that must succeed
  and list no machine. In place of the purge's match with its own worktree, its
  identity checks are the marker, which must be there as the guard requires it, and
  the state link, found from the path the marker records, whose target must be exactly
  that directory and which is made again if a restart removed it. It refuses, removing
  nothing, a state directory whose marker matches a worktree that exists, and names
  `just smolvm-purge` run from that worktree. `docs/how-to/develop-locally.md` says
  how to find an orphan, since each marker names its worktree, and how to remove it.
- **After the pin moves.** Facts for QS02 says what an old state directory can supply
  to a new release. The subsection on moving the pin in
  `docs/how-to/maintain-dependencies.md` says to run `just smolvm-purge` before the
  first boot under a new pin, and gives the order for both cases, since the purge
  needs a stamped install that matches the pin. Whoever moves the pin purges first,
  with the old pin and its stamped install still in place, and only then changes the
  pin and installs. A worktree that receives a moved pin with state already made
  installs the new release first and then purges, before any other machine command.
  Whether the guard also refuses a state directory made under another pin is in Open
  points. If it does, the purge still accepts such a state, since the purge is its
  recovery. Step 7 shows the refusal or the clean start, whichever the ruling chose.
- **A Rosetta preflight.** QS01 claim 3 and Facts for QS02 say that smolvm does not
  refuse to make a machine on a host without Rosetta, and name the read-only test
  that tells. Nothing in this ticket makes a machine before that test has passed on
  this host: step 3 runs it before its first machine and step 7 before the demo. A
  test that fails is a **stop** for the maintainer, since installing Rosetta is
  outside this ticket's authorisation. Whether the launcher runs the test itself is
  in Open points.
- **Nothing in the worktree holds machine state.** There is no `.smolvm/` directory
  and no `.gitignore` line for one. The worktree's id file is not machine state, and
  Open points says where it lies.
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
    it detects drift and does not make installs reproducible. Under ruling 9 it also
    says what the pin does not cover at run time, by pointing at Open point 7 of QS01's
    hand-back, and that egress stays as ruling 2 has it;
  - the guest's access under ruling 2 as amended: the worktree read-only at its own
    path, a few writable subdirectories that QS04 names, the common git directory
    read-only (ruling 7), open egress, host loopback and private ranges blocked. It
    states the consequence honestly, in both directions. The guest cannot write the
    worktree outside the writable subdirectories: not the `Justfile`, not the scripts,
    not `.git`, and not any ignored tree the host runs or loads. What it writes in a
    writable subdirectory is tracked or ignored as QS04 rules, and for tracked files
    the control is the maintainer reading the diff. The guest can read the whole
    worktree, except what ruling 6 hides, which QS04 lists and the maintainer rules
    on, and it can read the common git directory. It has open egress, so what it can
    read it can send out;
  - where the machine's state lives and what removes it (ruling 5): the state
    directory with the parents it is made under, its marker, the state link and the
    lock, each by path, and `just smolvm-purge`, with what the purge leaves behind,
    the lock's entry among it. It says that this state is outside the worktree, which
    is why a decision record is needed for it;
  - ruling 3 by name, with its ticket: wiring agents to Studio's MCP endpoint is
    QS05's, and QS05 adds the route it chose and what a caller can reach through it
    when it lands;
  - rulings 7 to 10 by name, each with the ticket that implements it: the wrapper
    around the guest's `git` (ruling 7), the searched host port (ruling 8) and the
    package snapshot (ruling 10) in QS04, with QS06 as ruling 10's follow-up, and what
    the pin does not cover (ruling 9) here and in QS03. QS03 and QS04 add their own
    particulars to the record when they land;
  - the supported host under ruling 4, and that CI never boots a machine, since no
    runner offers the hypervisor.

  Decision 0021's Context and Decision paragraphs on Studio are amended to point at
  0022, and the record is registered in `docs/manifest.yml` and
  `docs/decisions/README.md`.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 1 | The installed binary keeps its signature and entitlement after the installer's unpack | step 7, `codesign` read-only, and a boot |
| 2 | `just install-smolvm` over a stamped install skips, saying so, and reaches no network | step 8, under §11's denial with its negative controls, the skip message quoted |
| 3 | Under `just smolvm`, nothing new or changed outside the worktree, beyond what decision 0022 names, survives to the probe in any location QS01 step 3's probe can read. The probe does not see a file that was made and removed again before it ran. A location it cannot read, or whose listing ends in an error, is listed as unprobed, and the claim stays open for it | step 8, QS01 step 3's probe with its controls and its errors kept |
| 4 | A state root given through the state link is used as it is written: smolvm needs no alias of the kind QS01 claim 2 describes, and finds its machine again after the link is removed and made again | step 3, by hand, the kernel's sandbox log beside a negative control |
| 5 | A worktree mounted read-only at its own path, with a writable subdirectory mounted inside it, behaves as ruling 2 says: the guest writes only in the writable subdirectory, and cannot rename it or its parent | step 3 by hand, and step 7's demo through `just smolvm` |
| 6 | `just smolvm-purge` removes this worktree's state and nothing else, and refuses while a machine is left, and when the listing fails although the install is intact | steps 7 and 8, with `test ! -L` and `test ! -e` |

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
There are two checkpoints, at steps 4 and 10.

## Files touched

Each page row covers every sentence on that page that lists what `just initialize`,
gate 2 or the network recipes do, says where the machine's state lives, or says Studio
cannot be held by the repository. `.gitignore` gains no line for machine state, since
nothing in the worktree holds any, and `.tools/` already covers the install. Its one
row is conditional, for the worktree's id file.

| Path | Class | Content |
| --- | --- | --- |
| `tools/smolvm/smolvm.txt` | new | the pin file, with a comment naming decision 0022 and the bump how-to |
| `scripts/install-smolvm.sh` | new | the installer, POSIX `sh`, with `--check` |
| `scripts/check-smolvm-pin.sh` | new | the gate's check, POSIX `sh` |
| `docs/decisions/0022-quint-studio-machine.md` | new | the decision record |
| `Justfile` | shared, own lines | `install-smolvm`, `smolvm` and `smolvm-purge`, each with a comment; a line in `lock-check` and its comment |
| `.gitignore` | shared, own lines | only if the step 5 ruling puts the worktree's id file at a path that no line already ignores: one line for that file |
| `scripts/initialize.sh` | shared, own lines | only if the step 5 ruling keeps smolvm in `just initialize`: `just install-smolvm` after `just install-apalache`, on macOS arm64 only, with the skip message |
| `AGENTS.md` | shared, own lines | `just install-smolvm` and `just smolvm` in the list of recipes that reach the network |
| `docs/decisions/0021-project-managed-quint-cli.md` | shared, own lines | the Studio sentences in Context and Decision point at 0022 |
| `docs/decisions/README.md` | shared, own lines | 0022's row |
| `docs/manifest.yml` | shared, own lines | 0022's entry |
| `docs/how-to/develop-locally.md` | shared, own lines | what `just initialize` installs, or, if the ruling leaves smolvm out, that `just install-smolvm` is run by hand; where the machine's state lives, that `just smolvm-purge` removes it and what it leaves, and how to find and remove the state of a worktree that no longer exists; "Optional: Quint Studio" points at 0022 until QS04's page lands |
| `docs/tutorials/first-change.md` | shared, own lines | only if the step 5 ruling keeps smolvm in `just initialize`: its list of what `just initialize` installs gains smolvm, on macOS arm64 only |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | a subsection on moving the smolvm pin, which says to purge the state before the first boot under a new pin, in the order "After the pin moves" gives for each case; gate 2's description |
| `docs/reference/commands.md` | shared, own lines | `install-smolvm`, `smolvm` and `smolvm-purge` rows, the network list, and the `initialize` and `lock-check` rows |
| `docs/reference/configuration.md` | shared, own lines | a row for `tools/smolvm/smolvm.txt` and the pins table |
| `docs/reference/quality-gates.md` | shared, own lines | gate 2's row and the recipes that reach the network |
| `docs/explanation/security-model.md` | shared, own lines | smolvm checked by its pin; the guest's access, and the state kept outside the worktree, from decision 0022 |
| `docs/project/repository-map.md` | shared, own lines | `tools/smolvm/` and the two scripts |
| `.scratch/quint-studio/QS02-pinned-smolvm.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS02's Status cell |

## Steps

1. **Worktree, baseline and QS01's facts.** The maintainer names the base before this
   ticket starts (the README of these tickets, "Before QS01 starts"). Create the
   worktree on `ticket/qs02-pinned-smolvm` from that base, and record the base and its
   full commit id. Check that the base holds both QS01's hand-back, with its "Rulings
   after the hand-back" section, and the README of these tickets with rulings 5 to 10.
   If either is missing, **stop** and report: the particulars this ticket takes from
   QS01, and the rulings it builds, are not there to read. Run `just initialize`
   (network, authorised) and `just check` on the untouched worktree, and quote its last
   line. Confirm that 0022 is the next free decision number on the base. Read QS01's
   hand-back notes. Write `ai_tmp/qs02/amendments.md`: every point of "Design proposed"
   that a QS01 claim or fact contradicts, leaves open or supplies the particulars for,
   and the change it needs. Among them, each from the claim or section that "Design
   proposed" cites: which file of the release is run; the variables and options the
   guard refuses; the controlled environment; the limit on the root, and the state
   link's length against it; where the marker sits; the stamp's amendment for a first
   boot; the Rosetta test; and what an old state directory can supply after the pin
   moves, with each place in it that Facts for QS02 names for that.
2. **Read before writing** (network, authorised). Download the tarball and
   `checksums.sha256` into a new, empty `ai_tmp/qs02/download/`, compute the SHA-256,
   and compare it with the published line and with QS01's record. Never copy a hash from
   a page or a chat. Write `ai_tmp/qs02/plan.md`: the pin file's content, each script's
   and each recipe's outline against "Design proposed" as amended, the launcher with
   its guard, the lock, the purge and the orphan's entry point among them, a proposal
   for each item that Open points leaves to the ruling, the means by which step 7
   makes the purge's listing fail while the install is intact, decision 0022's outline,
   and the edit to each page in Files touched.
3. **The state root and the mounts, proved by hand** (writes outside the worktree,
   authorised by ruling 5: the state directory with the parents it is made under, the
   state link, and one directory made for the proof in `/tmp`, nothing else, and this
   step removes all but the parents again at its end; network, authorised: the image
   pull). QS01 ran neither ruling 5's state root nor ruling 2's mounts, so both are
   proved here, before the plan is reviewed and before anything is built on them.

   Unpack the tarball step 2 downloaded under `ai_tmp/qs02/`, as QS01 ran it; nothing
   is installed into `.tools/` yet. Before any machine is made, run the Rosetta test
   that QS01 claim 3 and Facts for QS02 name, read-only, and quote it with its exit
   status. If it fails, **stop** and report, as "A Rosetta preflight" says. Make the
   state directory and the state link by hand, at the paths the plan gives them. Run
   every smolvm command of this step in the controlled environment of section 9 of
   QS01's hand-back: the wrapper's environment, under a `sandbox-exec` profile that
   refuses every write outside `ai_tmp/qs02/`, the state directory and, for case (a),
   the proof's directory, with the profile's other allowances as section 9 has them.
   The profile names each of those directories by its physical path, as `pwd -P`
   prints it from inside the directory: the state directory is reached through the
   state link and the proof's directory through `/tmp`, and a rule written with a
   symbolic link in its path would not match the writes it is there to allow. Run the
   profile's controls before the first boot, as section 9 did, and three more with
   them: a write through the state link succeeds and lands in the state directory; a
   write in the proof's directory succeeds; and making a new name directly in `/tmp`,
   where an alias would be made, is refused and leaves nothing. Keep one machine at a
   time in a state root (QS01 claim 16 says why).

   Before the first case, record the length in bytes of each root the cases use, as
   it is written, beside the limit Facts for QS02 gives. The roots of (a) and (b) must
   be within the limit, and the root of (c), chosen as its bullet below says, must
   exceed it. No case runs until all three are so. A state link that does not fit is a
   defect in the name the plan gave it, and the plan is amended before the proof runs.

   A boot that succeeds is not the proof: an alias made silently in `/tmp` would also
   boot. The proof is the kernel's sandbox log, read as section 9 read it, beside a
   negative control. Each case makes a named machine from the image QS01 pinned, with a
   listener published to host loopback, not a bare boot, and gives the root the way
   claim 2 and Facts for QS02 name:
   - (a) a direct absolute root, short enough by the limit Facts for QS02 gives, in a
     directory made for the proof in `/tmp`;
   - (b) the state link as the root;
   - (c) the negative control: a root longer than the limit. That is the state
     directory's own absolute path if its length exceeds the limit. If it does not,
     it is a directory made for the control inside the state directory, under a name
     long enough that its path does, which this step's authorisation already covers.
     It must fail, and the log must show the refused alias write that QS01 claim 2
     describes, matched to that process and that time. It writes into the directory
     (b) uses, so it runs last, after every machine under the state link is deleted.

   For (a) and (b): the log shows no refused write by smolvm, no alias of that kind
   appears in `/tmp`, and a request through the published port is answered. For (b)
   also: smolvm names the machine's directory, read the way claim 2 read it, under the
   state link as it was written and not under the state directory's resolved path,
   while the files themselves lie in the state directory. That, beside (c)'s refusal,
   is what shows the root was used as written. Then stop the machine, remove the link,
   make it again, start the machine, and show the same machine in smolvm's own listing,
   a sentinel the guest wrote before the stop, and a forwarded request answered
   (claim 4).

   **The nested mounts** (ruling 2), on a machine under the state link: the worktree
   mounted read-only at its own path, and one writable subdirectory nested in it, a new
   directory under `ai_tmp/qs02/` mounted read-write at its own path. For this machine
   use the second profile of section 9, the one that allows writes under the whole
   worktree, with the state directory allowed as before, so that a refusal is the
   mount's and not the profile's, and quote each refusal's own message. From the guest:
   - a write in the writable subdirectory succeeds, and the host sees it;
   - a write to a tracked file fails;
   - opening the worktree's existing `.git` entry for writing fails, and, if the
     machine mounts the common git directory, so does creating a file in it;
   - a write to a path under an ignored tree other than `ai_tmp/` fails;
   - renaming the writable subdirectory fails, and so does renaming its parent.

   After each refusal the host's copy is unchanged, shown by a SHA-256 or a listing
   taken before and after (claim 5).

   Then delete every machine this step made. Remove the proof's directory, the state
   link and the state directory, and show each gone: `test ! -L` and `test ! -e` on
   the link, `test ! -e` on each directory. Step 7's recipe then starts from nothing.
   Write the results, with the log's lines, into `ai_tmp/qs02/plan.md` under a heading
   of their own, and amend the plan where they change it.

   If any part of the state-root proof fails, **stop** and report to the maintainer.
   The fallbacks to put to them, without choosing one, are the relative root inside
   the worktree that QS01 ran, hidden from the guest by an empty directory mounted
   over it (ruling 6), and a plain directory in `/tmp`, which a restart empties. If
   the nested mounts fail, **stop** and report in the same way, with the other two
   ways of section 3 of QS01's hand-back as the fallbacks. QS01 ran only the reverse
   nesting, read-only inside read-write.
4. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qs02/plan.md`, with step 3's results in it, and `ai_tmp/qs02/amendments.md`,
   before any tracked file changes). Questions:
   - Can the installer skip over a tree it did not unpack and check, or over one whose
     binary lost its signature? Does the stamp, as amended for a first boot, still
     catch a changed file?
   - Can any step of the installer re-sign the binary or strip an attribute, directly or
     through a tool it calls?
   - Can `just smolvm` run a binary other than the stamped one, or write machine state
     anywhere but the place decision 0022 names?
   - Does each `rm -rf` fail closed when its variable is empty?
   - Can a symlinked `.tools` or `.tools/smolvm` carry a write outside the worktree, or
     an inherited `tar` option leave a tree that is stamped but incomplete?
   - Can the guard, the purge or the orphan's entry point be made to follow, replace or
     remove something that is not this worktree's state: a symlink or a file at the
     state directory, a parent another user can write, a marker that is missing,
     symlinked or names another worktree, a name in `/tmp` that someone else made, or
     an entry swapped between its check and its use?
   - Does one invocation hold the lock from its check of the state to the end of
     smolvm's run, or of the purge's removal? Is the lock released however its holder
     ends, and can a machine that goes on running keep it?
   - Can the launcher print its sentinel without reaching smolvm, or reach smolvm
     without it? Does the means of failing the purge's listing fail the listing
     itself, and not a check that comes before it?
   - Can an inherited variable or an argument move smolvm's state, its registry
     credentials or its temporary files out of the controlled environment?
   - Does step 3's evidence show that the state link was used as it was written, or
     only that a machine booted?
   - Does decision 0022 state the read-only mount's consequence, and what the guest
     can read and send out, honestly?
5. **Red: the gate.** **Stop** first for the maintainer's ruling on "Design proposed" as
   amended and on each item Open points leaves to it, with step 3's results and the
   checkpoint's findings in hand, and record it. If the ruling changes the state
   directory's place or the state link's name, run step 3's state-root proof again with
   them, under that step's authorisation, before anything is built on them. Then add the
   pin file, `scripts/check-smolvm-pin.sh` and the `lock-check` line. Save the pin file
   to `ai_tmp/qs02/smolvm.txt.saved`. Show `just lock-check` passing. Then, each on an
   edit restored from the saved copy afterwards, show it failing and quote each message:
   `version=1.37` (two parts); the `sha256=` line removed; `sha256=` given 63 digits; an
   `asset=` naming another version; a fourth key added. After the last restore,
   `cmp tools/smolvm/smolvm.txt ai_tmp/qs02/smolvm.txt.saved` exits 0.
6. **The installer.** Add `scripts/install-smolvm.sh`, `just install-smolvm`, and, if
   the ruling keeps it, the line in `scripts/initialize.sh`. Then, quoting each run and
   its exit status and
   restoring each edit afterwards:
   - `just install-smolvm` (network, authorised) installs and writes the stamp;
   - a second run skips and says so;
   - one byte appended to a file under `.tools/smolvm/` that the stamp's digest covers:
     the installer replaces the tree (network, authorised);
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
7. **The recipes and the demo** (writes outside the worktree, authorised by ruling 5:
   the state directory with the parents it is made under, the state link, the lock,
   and, for the orphan's test alone, one more state directory and state link made by
   hand for a worktree path that does not exist, with the lock the entry point makes
   for them, nothing else; network, authorised: the image pull). Add `just smolvm` and
   `just smolvm-purge`. The orphan's apart, every fixture below that lies outside the
   worktree is made at the state directory's or the state link's own name, or inside
   the state directory, while no machine exists. Everything else a test makes, or
   points a fixture at, lies under `ai_tmp/qs02/`. Each fixture is removed afterwards.
   Each refusal of `just smolvm` exits 1, names its recovery, and prints neither the
   launcher's sentinel line nor a version line. Show, quoting each:
   - before the recipe has made any state: the state directory's name taken by a
     symlink to a new, empty directory, and separately by a regular file.
     `just smolvm --version` refuses each, the symlink's target stays empty, and the
     file is unchanged;
   - with the stamp in place, `just smolvm --version`, its standard output and its
     standard error kept apart: standard output holds the exact line QS01 recorded and
     nothing else, and standard error holds the launcher's sentinel line. The run has
     made the state directory, its marker, the state link and the lock: quote the
     paths, the marker, `ls -ld` of each, and the worktree's id file. With the stamp
     moved to `ai_tmp/qs02/stamp.saved`, it exits 1, naming `just install-smolvm`,
     with neither the sentinel nor the version line, so smolvm was never reached; with
     it restored, `cmp` exits 0. Quote the launcher's command that prints the sentinel
     and its command that runs the binary, the one straight after the other. `grep`
     shows that `scripts/install-smolvm.sh` holds no command that runs the installed
     binary, and lists every place that does: the launcher's one command, and the
     inventory's, if the purge and the orphan's entry point have a command of their
     own for it;
   - the marker, saved first to `ai_tmp/qs02/marker.saved` and restored after each
     case, with `cmp` exiting 0 at the end: one naming another worktree; one holding
     another id; none; and one replaced by a symlink to a copy of it. The recipe
     refuses each, and the state directory is otherwise unchanged;
   - the state link replaced by a directory, and separately by a link elsewhere, each
     with a sentinel tree in or behind it. The recipe refuses each, and the sentinel
     tree is untouched (a listing before and after). With the fixture removed, the
     next `just smolvm --version` makes the link again and passes, which is also what
     happens after a restart;
   - a value the caller set: each variable Facts for QS02 lists set in the
     environment, and each option given in `<args>`, pointing, where it takes a path,
     at a new, empty directory. The recipe refuses each, and the directory stays empty;
   - a `smolvm` stub first on `PATH` under `ai_tmp/qs02/` is not the one run;
   - `codesign --verify --strict` and `codesign -d --entitlements -` on the installed
     binary, read-only (claim 1);
   - the demo (network, authorised: the image pull). The Rosetta test of step 3 is run
     again first and quoted, with the same **stop** if it fails. The demo starts from a
     new install: a listing shows that `.tools/smolvm/` holds none of the files Facts
     for QS02 says smolvm writes beside its binary on first use. `just smolvm` boots the
     image QS01 pinned by digest, with `--rosetta`, the worktree mounted read-only at
     its own path, one writable subdirectory, a new directory under `ai_tmp/qs02/`,
     mounted read-write at its own path, and a listener QS01's facts name bound to
     `0.0.0.0` in the guest and published on `127.0.0.1:<port>`. The machine and its
     listener are started by commands that return, since a `just smolvm` command holds
     the lock until it ends. From the host, `curl -fsS http://127.0.0.1:<port>/`
     answers, and the guest lists a file the host created in the worktree a moment
     before. Then each probe of step 3's nested mounts again, with the same evidence
     (claim 5). Then, with the machine still running,
     `sh scripts/install-smolvm.sh --check` exits 0 and `just smolvm --version` runs
     again, so the first boot left the stamp good and a machine that is running does not
     keep the lock;
   - the purge refuses. With the demo's machine stopped and not deleted,
     `just smolvm-purge` exits 1, names the machine and how to delete it, and removes
     nothing. With the stamp moved aside, so that no inventory can be taken, which is
     also where a moved pin leaves a worktree until its release is installed, it exits
     1, naming `just install-smolvm`, and removes nothing, and the stamp is then
     restored, with `cmp` exiting 0. Started while a `just smolvm` command that was
     started first is still running, and so holds the lock, it removes nothing before
     that command ends, and that command ends as it does alone. After each, `test -L` on
     the state link and `test -d` on the state directory exit 0;
   - stop and delete the machine. Then an inventory that fails while the install is
     intact, made to fail by the means the plan named, which changes neither the
     stamped install nor any code. First `just smolvm`, given smolvm's own listing,
     exits non-zero under it, which shows that the means fails the listing itself.
     Then `just smolvm-purge` exits 1 and removes nothing, with
     `sh scripts/install-smolvm.sh --check` exiting 0 before and after it, and
     `test -L` and `test -d` as above. The means is then undone, and the listing
     passes again;
   - the pin's move. While no machine exists, plant a sentinel file at each place in the
     state directory that step 1's amendments name as one from which an old state
     directory can supply a new release, to stand for what an old release left there.
     Then show what the ruling chose: the refusal of a state directory whose marker
     records another pin, and after it the purge that still accepts that state, or the
     clean start after `just smolvm-purge`. Either way the purge runs with the pin and
     its stamped install in place, and the next `just smolvm --version` makes new state
     in which each planted path is absent (`test ! -e`), so the old state can supply
     nothing;
   - the orphan's entry point, on a fixture made by hand: a second state directory with
     a marker naming a worktree path that does not exist, and its state link. It
     refuses, removing nothing: with the marker missing; with the link pointing
     elsewhere; with a machine left in the fixture's state, made by hand with the
     stamped smolvm in the launcher's environment and the fixture's link as its root,
     and deleted in the same way after the refusal; and with the listing made to fail
     as above. It removes exactly the fixture's link and directory when all are
     right, and the fixture's lock, which it leaves, is then removed by hand. Given
     this worktree's own state directory, whose marker matches a worktree that
     exists, it refuses, names `just smolvm-purge` and removes nothing. This
     worktree's own state is unchanged throughout (a listing before and after).
8. **No network, and nothing outside the worktree but the state** (writes outside the
   worktree, authorised by ruling 5: the state directory with the parents it is made
   under, the state link and the lock, nothing else; network, authorised: the image
   pull). Run `just install-smolvm` under §11's macOS denial,
   `sandbox-exec -p '(version 1)(allow default)(deny network*)'`, with both of §11's
   `curl` negative controls failing inside the same denial, and quote all three exit
   codes and the installer's own message that it skipped (claim 2). Exit 0 without that
   message is not a pass: an installer that tried the network, failed and went on would
   exit 0 too. Start from no state, running `just smolvm-purge` if any is left, so that
   the probe sees the recipe make it. Then rerun step 7's demo with QS01 step 3's probe,
   its marker under `ai_tmp/qs02/`, with its controls and its unreadable directories
   recorded. The probe is this ticket's own script under `ai_tmp/qs02/`, written from
   QS01's step 3 and section 9, and it keeps what the probe they describe let go: each
   root's listing is taken with its exit status and its errors in a file, and no
   pipeline carries a count. Show that nothing new or changed outside the worktree
   survives to the probe beyond what decision 0022 names, the state directory with its
   parents, the state link and the lock, in any location the probe can read. List each
   location it could not read, and each whose listing ended in an error, as unprobed,
   where claim 3 stays open (claim 3). The probe shows what is there when it runs, and
   not a file that was made and removed again before then, and the hand-back says so.
   The stronger evidence is step 3's profile, which refused and logged every write
   outside the directories it allowed, but it covers only the commands step 3 ran by
   hand and not this bracket. The state directory and the state link must themselves
   appear in the probe's output, which shows that the probe reads where they lie. Then
   stop and delete the machine and run `just smolvm-purge`. Quote what it printed, then
   `test ! -L` and `test ! -e` on the state link and `test ! -e` on the state directory,
   each with its exit status, and a listing of the state directory's parent taken before
   and after, which differs by this worktree's state directory alone (claim 6). `ls -ld`
   shows the lock's entry still there.
9. **Words.** Write decision 0022 and edit each page in Files touched. Run
   `just check-docs` and `just check-agents`.
10. **Adversarial review** (checkpoint 2: conformance and breakage, over the diff with
    the pages included, before the final commit). Questions:
    - Does `lock-check` stay offline and read-only, and does `just check` stay free of
      the network and of smolvm?
    - Could Verification pass while `just smolvm` ran an unstamped binary, or while it
      left state behind outside the worktree? Could it pass a refusal in which smolvm
      was reached all the same?
    - Can `just smolvm-purge`, or the orphan's entry point, remove anything but the
      state it validated?
    - What does `just initialize` do on Linux, and in a worktree where the download
      fails?
    - Does every page agree on where the pin lives, what checks it, where state lives
      and what removes it, and what the guest can reach, read and write? Does any page
      outside Files touched still say Studio cannot be held by the repository?

    The README of these tickets, "Rules every ticket holds", binds this ticket from
    its baseline to its hand-back, and what it says of a fix made at this checkpoint
    is done before step 11's record commit.
11. **Commit and hand back** (writes outside the worktree, authorised by ruling 5:
    Verification's `just smolvm --version` makes the state directory with its parents,
    the state link and the lock, and its purge removes the state directory and the state
    link again and leaves the lock's entry). Commit the work. Rerun Verification and
    quote it, headed by the full commit id. Set `status: done` here and in
    `.scratch/quint-studio/README.md`, write the hand-back notes, and commit the record.
    On the record commit, run `git diff --name-only HEAD~1 HEAD`, which must list only
    this ticket and the README, then `just check` and `git status --short`, and report
    their output to the maintainer with the hand-back, headed by that commit, since a
    commit cannot quote its own check. If any fails, fix the cause in a new commit and
    run all three again. **Stop** before pushing or opening a pull request: both need
    authorisation (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `tools/smolvm/smolvm.txt` names `version=1.37.4`, the asset QS01 recorded and the hash
  step 2 computed, equal to the release's own line; each mutation of step 5 makes
  `just lock-check` fail, each quoted.
- `just install-smolvm` installs, skips a stamped install, replaces a changed, unstamped
  or non-executable tree, refuses a wrong hash leaving nothing behind, installs the full
  tree despite an inherited `TAR_OPTIONS`, refuses a symlinked `.tools` or
  `.tools/smolvm` leaving its target unchanged, and refuses another host, each quoted.
- The installed binary is signed and holds the Hypervisor entitlement, quoted.
- The Rosetta test is quoted with its exit status before step 3's first machine and
  before step 7's demo.
- Step 3's state-root proof is recorded (claim 4): the length of each of the three roots
  beside the limit, with (a) and (b) within it and (c) over it; cases (a) and (b) with
  no refused write by smolvm in the kernel's sandbox log and no alias in `/tmp`; case
  (c) failing with the refused alias write matched to its process and time; and, after
  the link was removed and made again, the same machine, its sentinel and a forwarded
  request answered. The proof's directory, the state link and the state directory that
  step 3 made are each shown gone. If the proof failed, the stop and the maintainer's
  ruling are recorded in its place.
- The nested mounts are shown twice (claim 5), by hand at step 3 and through
  `just smolvm` in step 7's demo: the write in the writable subdirectory seen on the
  host, and each refused write and rename quoted with the host's copy unchanged. If
  they failed, the stop and the maintainer's ruling are recorded in their place.
- `just smolvm` refuses without the stamp; on a symlink or a regular file at the state
  directory; on a marker that is missing, symlinked, or names another worktree or
  another id; on a directory, or a link elsewhere, at the state link; and on a value
  the caller set for one of smolvm's own variables or options. Each exits 1 with its
  diagnostic, without the launcher's sentinel line and with no start of smolvm, and
  each fixture's target or sentinel tree is unchanged. It never runs a `smolvm` from
  `PATH`, makes an absent state link again, keeps its state where decision 0022 says,
  and puts nothing of its own on standard output.
- The demo boots from a new install, mounts the worktree read-only with one writable
  subdirectory and answers on host loopback, and `--check` passes after that first
  boot, quoted.
- `just smolvm-purge` refuses, removing nothing, while a machine is left, when no
  inventory can be taken for want of the stamp, and when the listing fails with the
  install intact. It removes nothing while a `just smolvm` command that is still
  running holds the lock, and a machine that is running does not keep the lock. With
  no machine left it removes the state link and the state directory, shown by
  `test ! -L` and `test ! -e`, and nothing else in the state directory's parent, and
  the lock's entry is still there (claim 6).
- The orphan's entry point refuses a missing marker, a link that points elsewhere, a
  machine left, a listing that fails, and a state directory whose worktree exists,
  removing nothing each time, and removes exactly its fixture's link and directory.
- After a move of the pin, what the ruling chose is shown: the refusal, with the
  purge that still accepts that state, or the clean start. Each planted path is
  absent from the new state.
- `just install-smolvm` over a stamped install exits 0 and prints its skip message
  under the network denial, with both negative controls failing.
- Step 8's probe shows nothing new or changed outside the worktree surviving to it
  beyond what decision 0022 names, with the state directory and the state link in its
  output, and each location it could not read, or whose listing ended in an error,
  listed as unprobed (claim 3).
- `AGENTS.md` lists `just install-smolvm` and `just smolvm` among the recipes that reach
  the network, and `just check-agents` passes.
- Decision 0022 exists, is registered, and records the rulings as the README of these
  tickets states them: the guest's access under ruling 2 as amended, with what the
  guest cannot write and what it can read and send out; the state outside the
  worktree, what removes it and what the purge leaves (ruling 5); and rulings 3 and 6
  to 10, each with the ticket that implements it. Decision 0021 points at it.
- `docs/how-to/develop-locally.md` says where the state lives, how to purge it, what
  the purge leaves, and how to find and remove an orphan; `docs/reference/commands.md`
  has the `smolvm-purge` row; and `docs/how-to/maintain-dependencies.md` says to purge
  when the pin moves, in the order for each case.
- At the hand-back the state link and the state directory are gone, and anything else
  this ticket made outside the worktree and left there, the lock's entry among it, is
  listed by path.
- Each claim has its outcome recorded.
- Both checkpoints are recorded as §14's Record asks, with AP01's deltas.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record
  commit, as step 11 reports.

## Verification

Run from a file as one `sh` script; `set -eu` stops it at the first command that fails.
No pipeline carries a result: each command writes to a file, its exit status is checked,
and the file is matched exactly.

```sh
set -eu
git rev-parse HEAD
version_line='<the exact smolvm --version line QS01 recorded>'
reached_line='<the sentinel line the launcher prints before it runs smolvm, quoted at step 7>'
state_dir='<the absolute path of the state directory, quoted at step 7>'
state_link='<the path of the state link, quoted at step 7>'
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
grep -Fx "$reached_line" "$out/version.log"
grep -Fx "$version_line" "$out/version.log"
test -d "$state_dir"
test -L "$state_link"

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
if grep -Fx "$reached_line" "$out/refuse.log"; then exit 1; else test "$?" -eq 1; fi
if grep -Fx "$version_line" "$out/refuse.log"; then exit 1; else test "$?" -eq 1; fi
grep -Fx "archive=$pin" "$stamp"

# The purge removes this worktree's state; the link is tested without following it.
just smolvm-purge >| "$out/purge.log" 2>&1
test ! -L "$state_link"
test ! -e "$state_link"
test ! -e "$state_dir"

just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

`version_line` is filled in from QS01's hand-back, and `reached_line`, `state_dir` and
`state_link` from step 7's quotes, before the script runs. The green run of
`just smolvm --version` is the positive control for the refusal's silent probes: the
launcher's sentinel line and the version line, each matched whole, appear when smolvm
is reached and must both be absent when the guard refuses. The binary is signed and
cannot be wrapped, so the sentinel is a line the launcher prints straight before the
one command that runs it, and not a shim; step 7 quotes the two commands and shows
that the installer never runs the binary. The EXIT trap restores the pin file and the
stamp if any line fails between a mutation and its restore. That green run also makes
the state outside the worktree, which ruling 5 authorises, and the script's last act
on it is the purge, so a run that passes leaves neither the state link nor the state
directory. The lock's entry stays, as decision 0022 says. If the script stops before
the purge, run `just smolvm-purge` by hand. Steps 3 and 5 to 8 hold the proofs, the
other mutations, the refusals over the state, the demo and the network denial, whose
output is quoted there.

## Open points

- Whether `just initialize` installs smolvm at all, or leaves it to a worktree that runs
  Studio. The design proposes it does, on macOS arm64; the maintainer rules at step 5.
- If QS01 claim 14 found smolvm on conda-forge or crates.io, whether that route replaces
  this installer. This ticket does not cover it: its design and Files touched assume
  neither route, so the maintainer rules on the route and the ticket is rewritten
  before step 2.
- If step 3's state-root proof fails, which fallback replaces ruling 5's design. The
  two this ticket puts to the maintainer are the relative root inside the worktree
  that QS01 ran, hidden from the guest by an empty directory mounted over it
  (ruling 6), and a plain directory in `/tmp`, which a restart empties. The ticket is
  rewritten for the one chosen before it goes on.
- If step 3's nested mounts fail, which of the other two ways of section 3 of QS01's
  hand-back replaces ruling 2's mounts. That choice reaches QS04 as well.
- What the design leaves to the ruling at step 5. Step 2's plan proposes an answer to
  each, and the builder settles none of them alone:
  - where the launcher's code lives, its guard, the lock and the controlled
    environment with it: in the `smolvm` recipe's own body, written so that it is
    still one invocation, or in a script of its own, which would join Files touched.
    QS04's script runs smolvm through it, and from a recipe's body a script reaches
    it only through `just`, whose own lines then come out on standard error. QS05's
    design calls it directly, without `just`, and needs nothing but the protocol on
    its standard output, so a ruling for the recipe's body alone sends QS04 and QS05
    back to their own rulings. Wherever it lives, `scripts/install-smolvm.sh` still
    never runs the binary;
  - the path of the worktree's id file. It must be ignored, and it must lie outside
    `.tools/smolvm/`, which the installer replaces and the stamp certifies. A path
    that `.gitignore` does not already cover needs the conditional `.gitignore` row
    of Files touched. Also whether the purge removes it, and whether a new state
    directory takes an id that is already there or a new one. QS04 ties the directory
    that holds Studio's sign-in to this id, so an id that the purge removes, or that
    is made anew, cuts that data off from its worktree: the hand-back says which
    happens;
  - the lock: its path and its mechanism, which must give what "One lock" asks:
    release however the holder ends, nothing kept by a machine that goes on running,
    and an entry that the orphan's entry point can find from the orphan's state
    directory alone. Also whether a second caller waits or is refused;
  - a command that runs for a long time. The launcher holds the lock until smolvm's
    run has ended, so a command that lasts as long as an agent's session, which is
    what QS05's launcher would run, holds it for that long, and meanwhile every other
    operation on this worktree's state waits or is refused: `just smolvm`, the purge
    and QS04's actions. Whether that is accepted, or such a command is given another
    rule, is for the ruling, and QS05 builds on the answer;
  - the purge: whether it removes the parents it leaves empty; what it does when
    there is no state, and when the state fails its checks; and the form of the
    orphan's entry point, an argument to `just smolvm-purge` or a recipe of its own,
    which would change the `Justfile` and `docs/reference/commands.md` rows;
  - the orphan's leftovers: how a machine that the orphan's inventory lists is
    deleted, since `just smolvm` reaches this worktree's state and no other; and what
    removes the lock's entry of a worktree that no longer exists, since neither the
    purge nor the entry point does;
  - the stamp: whether its digest also covers each entry's type, each symbolic link's
    target and the file modes, and whether `--check`, and so every `just smolvm`,
    also verifies the signature and the entitlement, read-only, on every run. As
    proposed the digest covers the files' paths and contents, the binary's execute
    bit is tested beside it, and the signature and the entitlement are checked when
    the tree is installed. So a link pointed at another file with the same content,
    or a helper that lost its execute bit, would pass `--check`;
  - the Rosetta preflight: whether the launcher runs the test itself and refuses a
    command that would make a machine under Rosetta, which needs it to read `<args>`
    for that as well, or leaves the test to whatever makes a machine, as steps 3 and 7
    here run it by hand and QS04's `up` would run it;
  - the place of the temporary directory the controlled environment names: an ignored
    tree in the worktree, as QS01 had it, or the state directory;
  - after the pin moves, whether the guard refuses a state directory made under
    another pin, which needs the pin recorded in the marker, or whether the how-to's
    purge is the whole rule;
  - `AGENTS.md` says that authority stops at the worktree, and after this ticket
    `just smolvm` writes outside it, where decision 0022 says. Files touched gives
    `AGENTS.md` only the two recipes in its list of those that reach the network.
    Whether that sentence of the agent contract is amended to name decision 0022's
    paths, and by which ticket, is the maintainer's.

## Hand-back notes

Every quote below ran at `2565d3d5a5c1605f835377defcea17340158911a` with uncommitted
changes (this ticket's work) unless it names another commit. Raw logs are under
`ai_tmp/qs02/logs/` and the review files under `ai_tmp/review/QS02-1/` and
`ai_tmp/review/QS02-2/`; none is committed.

### 1. Base and branch

- Base `quint-studio-smolmachine` at `2565d3d5a5c1605f835377defcea17340158911a`, which
  the maintainer named when QS01 was merged into it on 2026-11-25. Step 1 checked that
  it holds QS01's hand-back with "Rulings after the hand-back" (line 3306) and the
  README of these tickets with rulings 5 to 10.
- Branch `ticket/qs02-pinned-smolvm`, worktree
  `/Users/scutting/.supacode/repos/pachinko/QS02`, made with
  `supacode repo worktree-new`. Hooks were not installed from it; the primary
  checkout's hooks ran at each commit.
- Decision 0022 was the next free number (`docs/decisions/` ended at 0021).

### 2. Baseline

After `just initialize` (exit 0; network, authorised by step 1), on the untouched
worktree at `2565d3d5a5c1605f835377defcea17340158911a`:

```text
$ just check
All checks passed and the worktree is unchanged.
check exit 0
$ git status --short
status exit 0
```

### 3. QS01's facts and the amendments

`ai_tmp/qs02/amendments.md`, A1 to A12, holds every point of "Design proposed" that
QS01's hand-back contradicted, left open or supplied, with its source. These changed
the design:

- **A1, the file that is run.** It is the release's `smolvm` bash script, as QS01's
  Facts for QS02 say ("Run the 'smolvm' script (not smolvm-bin directly)"). Both
  `codesign` reads name `smolvm-bin`. `/bin/sh` here is bash 3.2, which imports an
  exported function in place of any command, even `cd`. So all three scripts fix
  `PATH` on their first line and refuse an exported function on the next.
- **A2, what the guard refuses:**
  - any `SMOLVM_*` or `DYLD_*` variable, `XDG_CACHE_HOME`, `XDG_DATA_HOME`,
    `XDG_CONFIG_HOME` and `DOCKER_CONFIG`;
  - `--state-dir` before `--`;
  - a `HOME` other than the one in the user database.

  Proxy variables are cleared by `env -i`, not refused.
- **A3, the environment.** `env -i` with `PATH`, `HOME`, `TERM`, `TMPDIR`,
  `DOCKER_CONFIG` and `SMOLVM_DATA_DIR`. The last four name the state link as written,
  or directories inside the state.
- **A4, the root's limit.** 42 bytes as written. The state link
  `/tmp/pachinko-smolvm-e906b65b` is 29 bytes, and the state directory's own path 60.
- **A5, the marker** `.pachinko-state` sits at the state root, where QS01 claim 16
  showed smolvm leaves a host file alone.
- **A6, the stamp.** The tarball holds 938 entries: 423 files, 131 directories and 384
  symbolic links, 266 of them absolute guest paths.
  - AP02's digest refuses a link, so it could not be reused. This digest covers each
    entry's type, mode, link target and content, and never follows a link.
  - It leaves out the stamp, and the two first-use templates by exact name at the top
    level.
  - Extraction is `tar -xpzf`. Plain `-xzf` dropped `agent-rootfs/tmp` and
    `var/tmp` from `1777` to `0755`.
- **A7, the Rosetta test.** `test -e /Library/Apple/usr/libexec/oah/RosettaLinux/rosetta`.
  It exited 0 before step 3 and before each demo.
- **A8, the image and the listener.** The image is `debian:trixie-slim`'s amd64
  manifest digest. QS01 named no guest listener except Studio, so the demo's listener
  is a one-line Perl server (`perl-base` is Essential in Debian). Ports came from
  7101 to 7120, each shown free by `lsof` beforehand, never 7000.
- **A9, what an old state can supply after the pin moves.** Templates under `.smolvm/`
  and `Library/Caches/smolvm/`, and the database, machines, image seeds and registry
  tokens.
- **A12, the lock.** `lockf(1)` on a read-only descriptor that the launcher's shell
  holds.

### 4. The state root and the mounts, proved by hand

`ai_tmp/qs02/plan.md`, "Step 3's results" and 3b to 3d. Every smolvm command in it
ran through `ai_tmp/qs02/bin/smolvm-hand` under `sandbox-exec`. The profiles came from
QS01's section 9 and named physical paths. Their controls were run before any boot:
writes under `ai_tmp/qs02/` and through the state link succeeded, and writes to
`$HOME`, `/private/tmp`, `/tmp/ks-…`, the real `$TMPDIR`, `/Users/Shared` and the
worktree root were refused. 3d ran them again under `noproof.sb` with every check
asserted.

- **(a) direct root `/tmp/pq02-a` (11 bytes) and (b) the state link (29 bytes).** Each
  case was a named machine from the amd64 digest, with `--rosetta --block-private
  --deny-cidr 127.0.0.0/8 -p <port>:8080`.
  - Each printed `x86_64`. Each answered `qs02-ok`, compared byte for byte in 3d.
  - `lsof` showed the published port on `127.0.0.1` and `[::1]` only, owned by the VMM.
  - `/private/tmp` was listed with its status before, during and after; it held no
    `ks-` name.
  - For (b), `machine data-dir` printed
    `/tmp/pachinko-smolvm-e906b65b/Library/Caches/smolvm/vms/…`, under the link as
    written, while `find` listed the files under the state directory's physical path.
- **Claim 4, the relink.** It was shown in step 3 under `deny.sb` and in 3d under
  `noproof.sb`.
  - The guest wrote `/root/qs02-sentinel` and the machine was stopped.
  - The link was removed (`test ! -L` and `test ! -e`, exit 0) and made again.
  - `machine ls` listed the machine `stopped`, and it started again.
  - The sentinel read back, `data-dir` named the same directory, and the reply was
    `cmp`-equal.
- **(c) the state directory's own path (60 bytes), run last.** It failed at the alias:
  `cannot address Unix socket …/agent.sock: Operation not permitted (os error 1) at path
  "/tmp/ks-wVhfwG"`. It failed the same way:
  - as an ephemeral `machine run`;
  - as a named machine with `-p` under `deny.sb`, at `/tmp/ks-8BuonM` (pid 77559);
  - under `noproof.sb` (pid 94212).

  No `ks-` name was left in `/private/tmp` after any of these.
- **The kernel's log did not record these refusals.** Neither `log show` nor a live
  `log stream` showed them, inside or outside the Bash tool's sandbox. A control in
  which a refusal was certain (smolvm's own database `mkdir` under a root the profile
  forbade) printed only `1 duplicate report for Sandbox: smolvm-bin(65297) deny(1)
  file-write-create …`. SBPL rejects `(with report)` on a deny. So the proof rests on
  smolvm's own error, libkrun's `SocketPath::new`, (a) and (b) booting under the same
  refusal, and the relink. The maintainer ruled on this (Q1; section 5).
- **Nested mounts, claim 5** (3b, `qs02-m3`): the worktree `-v W:W:ro`, `ai_tmp/qs02/rw`
  read-write, and the common git dir read-only. `/proc/mounts` showed them `virtiofs ro`,
  `rw` and `ro`.
  - The guest's write in `rw` reached the host.
  - Seven commands were refused with `Read-only file system`: writes to `Justfile`,
    `.git`, a file in the common git dir, `.tools` and `target`, and the renames of
    `rw` and of `ai_tmp/qs02`.
  - After each refusal, the host's state was compared with the snapshot before it
    (hashes of `Justfile` and `.git`, and six listings, every producer's status
    checked); `cmp` exited 0 every time.
  - The profile there allowed the common git dir, so that its refusal had to come
    from the mount (Departure 3).
- **The lock** (3c). A contender got `lockf` exit 75 while a holder's
  `machine exec -- sleep 8` ran, and `lsof` named the holder. A VMM started by
  `machine start` held no copy of the descriptor. But `kill -9` of the holder freed the
  lock while its smolvm client still ran: this became Q5.
- **Removal.** Every machine was deleted and the proof's directory, the state link and
  the state directory were removed, each shown gone. Only the two parents remained.

### 5. Ruling

`ai_tmp/qs02/ruling.md`, 2026-11-26. It was asked through the question tool after the
brief `ai_tmp/qs02/ruling-brief.md`, with step 3's results and both rounds of
checkpoint 1 in hand. The maintainer accepted every proposal, each answer marked
"(Recommended)":

- Section A, 14 proposals: "Accept all 14". Among them:
  - `just initialize` installs smolvm;
  - `scripts/smolvm.sh` is a script of its own;
  - the id file is `.tools/smolvm-state-id`;
  - the lock is `lockf`, refuses at once and is never removed;
  - the purge leaves the parents;
  - the digest covers type, mode and link target, and `--check` runs `codesign` each
    call;
  - callers run the Rosetta test;
  - `TMPDIR` and `DOCKER_CONFIG` live in the state;
  - the marker records the pin;
  - the A2 refusals;
  - ACLs: Q8;
  - the extra guard checks: Q12, with fixture (a);
  - the extra fixtures: Q13;
  - the refusal evidence: Q14.
- Q1 "Accept, narrower claim"; Q2 "AP02's model, stated"; Q3 "Exclude, say so"; Q4
  "Install lock"; Q5 "Trap and wait"; Q6 "Accept, name the holder"; Q7 "State + QS04
  only"; Q9 "Allow ls/stop/delete"; Q10 "Amend in QS02"; Q11 "--delete argument".

The state directory's place and the state link's name did not change, so step 3 was
not rerun.

### 6. Claims

| # | Outcome |
| --- | --- |
| 1 | **Holds.** `codesign --verify --strict .tools/smolvm/smolvm-bin` exit 0, and the three entitlements, `com.apple.security.hypervisor` among them, after each install. Every demo booted. |
| 2 | **Holds.** Under `sandbox-exec -p '(version 1)(allow default)(deny network*)'`, `just install-smolvm` exit 0, printing `install-smolvm: smolvm 1.37.4 is installed and matches its stamp; nothing to do.` The `curl` controls exited 6 (`Could not resolve host: static.rust-lang.org`) and 7 (`Failed to connect to 1.1.1.1 port 443`). |
| 3 | **Holds, for what the probe could read.** Section 10. The probe sees only what was present when it ran, not a file made and removed before then. |
| 4 | **Holds in the narrower form the maintainer accepted (Q1):** the state link, within the limit, made no fatal alias attempt, and the machine was found again after the link was made again. "Needs no alias at all" is not shown; the kernel's log did not report smolvm-bin's refusals here. |
| 5 | **Holds.** By hand at step 3 (3b) and through `just smolvm` in step 7's demo, run three times: section 9. |
| 6 | **Holds.** Sections 9 and 10: the purge refuses, removing nothing, with a machine left, without the stamp, while another command holds the lock, and when the listing fails with the install intact. Otherwise it removes the link and the directory (`test ! -L`, `test ! -e`), and the parent's names are unchanged apart from them. |

### 7. What landed

Work commit `58a0f1bf6fce8fdf67cfac8866055a6700db8e54`, and
`366ef2164b7624d83f8228972c12bf3b16c240f9` with checkpoint 2's second-round fixes
(section 12):

- **New files:**
  - `tools/smolvm/smolvm.txt`;
  - `scripts/check-smolvm-pin.sh`, `scripts/install-smolvm.sh` and `scripts/smolvm.sh`,
    each mode 0755;
  - `docs/decisions/0022-quint-studio-machine.md`.
- **The `Justfile`:** `install-smolvm`, `smolvm` and `smolvm-purge`, and the
  `lock-check` line. Each script is run as `/usr/bin/env -u SHELLOPTS -u BASH_ENV -u
  ENV -u PS4 /bin/sh scripts/…`.
- **`scripts/initialize.sh`:** smolvm on macOS arm64; elsewhere a skip message naming
  decision 0022.
- **`AGENTS.md`:** `just install-smolvm` and `just smolvm` join the network list, and
  the authority paragraph names decision 0022's paths as its one exception.
- **The pages in Files touched,** decision 0021, `docs/manifest.yml` and
  `docs/decisions/README.md`.
- **Beyond Files touched** (Departure 6): `docs/reference/testing.md`,
  `docs/reference/agent-contract.md` and `docs/operations/maintenance.md`, one stale
  sentence each.

### 8. The gate and the installer, red and green

- **Pin check** (`logs/final2/pin.txt` and `pin-lock-check.txt`, failures 0, at
  12:26 on the code of `366ef21`). Through
  `just lock-check`:
  - green on the pin as committed;
  - each mutation fails with its own `tools/smolvm/smolvm.txt: <key>:` message:
    `version=1.37`, `sha256=` removed, 63 digits, `asset=` naming 1.37.3, a fourth key,
    a repeated key, uppercase hex;
  - a whitespace-only line passes, and a symlink at `tools/smolvm` or the pin is
    refused;
  - `cmp` with the saved pin exits 0.
  - Under `/bin/dash`, CI's `/bin/sh` on Linux (`logs/final/dash-pin.txt`): the check
    exits 0 on the pin and every mutation fails with the same message. The one
    difference is the exported-function case, which passes there: dash imports no
    function and drops a variable named `BASH_FUNC_cd%%` from what it passes on, so
    nothing is replaced and nothing is seen.
- **Installer** (`logs/final2/install.txt`, 19 cases, failures 0; red runs in
  `step6-red*.txt`):
  - installs and stamps; skips a stamped tree;
  - replaces the tree after an appended byte, a removed stamp, `chmod a-x` on either
    file, a changed mode or a changed link target;
  - a changed `sha256=` fails, printing both hashes, and leaves no tree and no
    temporary directory;
  - `TAR_OPTIONS` excluding a file still gives the full tree (macOS's bsdtar ignores
    it);
  - the templates' exclusion is exact;
  - refuses a symlinked `.tools/smolvm` and a symlinked `.tools`, with each target
    unchanged and `.tools` restored exactly;
  - the host check (Q12 (a)), on a fake root with a one-line `diff`: `Linux`/`x86_64`
    is refused, naming ruling 4 and decision 0022; `Darwin`/`arm64` passes;
  - refuses an exported function, and a busy install lock;
  - checksums are computed by the pixi Python, and `curl` gets `-q`;
  - `--check` exits 0 at the end.

### 9. The recipes and the state, red and green

`ai_tmp/qs02/logs/final2/`, from the rerun after both rounds of checkpoint 2's fixes,
on the code of `366ef21` (2026-11-26 12:28 to 12:33, every part asserted, all passed;
`logs/final3-chain.txt`):

- **The launcher's refusals** (`launcher.txt`, 46 cases, failures 0):
  - Before any state, a symlink or a regular file at the state directory's name is
    refused, and the target stays empty or unchanged.
  - The first `just smolvm --version` prints exactly `smolvm 1.37.4` on stdout and the
    sentinel on stderr. It makes the state directory, its marker (path, id, pin),
    `tmp/`, `docker-config/`, the state link and the lock, and the fresh state holds no
    symbolic link.
  - These are refused, each exit 1, with neither the sentinel nor the version line,
    and with each fixture's target unchanged:
    - no stamp;
    - a marker naming another worktree, holding another id, a bad id or a repeated
      key, missing, or a symlink;
    - the state link as a directory, or pointing elsewhere;
    - each of the ten variables, and `--state-dir` and `--state-dir=`;
    - a foreign `HOME`, or an exported function;
    - a link inside the state, or a link at the lock's name;
    - the install lock or the state lock busy, naming the holder's pid;
    - a non-empty `docker-config/`;
    - `pachinko/smolvm` at 0755, after which it was restored to 0700;
    - `--delete` empty, or starting with `-`.
  - An absent state link is made again. A `smolvm` stub first on `PATH` is not run. An
    empty state directory is made again, and a missing `tmp/` is repaired.
  - Case 43 is a recorded limit, not a pass (Open points).
- **Signals, ACLs and identity** (`extra.txt`, all passed):
  - `TERM` sent to the launcher's shell while its `machine exec -- sleep 10` ran: the
    launcher kept running, a contender was refused for the lock, and the launcher ended
    after smolvm (exit 0), after which the contender ran.
  - `kill -9` of the launcher's shell alone: its smolvm client still ran (`pgrep` exit
    0) and a contender ran, the one early release decision 0022 states.
  - `everyone allow add_file` on `~/Library/Caches/pachinko/smolvm`: refused, naming
    `chmod -N`; the ACL then removed, `ls -lde` equal to before.
  - An orphan whose recorded worktree lies behind a parent of mode 0400: refused,
    "cannot tell whether …"; `test ! -e` on that live worktree exits 0, which is why
    it is not used. Searchable again: refused as live. With the worktree removed: purged.
  - A state directory holding only an unlinked `.pachinko-state.new`: removed and made
    again, with a whole marker.
  - An orphan whose recorded worktree has a symlinked `.tools`: refused, nothing
    removed.
- **The demo** (`demo.txt`). It ran from a new install, which held neither template.
  `just smolvm` created and started `qs02-demo` with the worktree read-only, `rw-demo`
  read-write and `-p 7120:8080`.
  - Uname `x86_64`; the reply `cmp`-equal; `lsof` showed `127.0.0.1:7120` and
    `[::1]:7120` only.
  - The guest listed a file the host had just made, and its write in `rw-demo` reached
    the host.
  - Writes to `Justfile`, `.git`, `.tools` and `target`, and renames of `rw-demo`, its parent
    `ai_tmp/qs02` and `ai_tmp`, were refused with `Read-only file system`.
    `Justfile` and `.git` were unchanged (`cmp`).
  - With the machine running, `--check` exited 0 with both templates now present, and
    `just smolvm --version` ran: the running machine kept no lock. No symbolic link
    was in the state after the boot or while it ran.
- **The purge's refusals** (`purge-refusals.txt`), each removing nothing:
  - a stopped machine is named, with how to delete it;
  - without the stamp, it names `just install-smolvm`; `--check` exited 0 after the
    stamp was restored;
  - while a `just smolvm machine exec -- sleep 10` ran, it was refused for the lock,
    and that command ended with exit 0.
  
  `test -L` and `test -d` exited 0 throughout.
- **The listing that fails** (`listing.txt`), with the install intact:
  - `just smolvm machine ls --json` printed `[]`;
  - after `chmod 000` on `…/server/smolvm.db` the same listing exited 1 after the
    sentinel, with `unable to open database file`;
  - `just smolvm-purge` exited 1 and removed nothing, with `--check` 0 before and
    after;
  - the mode was restored and the listing printed `[]` again.
- **The pin's move** (`pin.txt`). Six sentinels were planted: four template paths, one
  under `Library/Application Support/smolvm`, one under `Library/Caches/smolvm/vms`.
  The marker was changed to record another pin.
  - `--version` was refused ("another smolvm pin"), while `machine ls` and
    `machine delete` ran (Q9).
  - The purge refused while `qs02-old` was left, then accepted the old-pin state: the
    state directory and the link were gone.
  - The next `--version` made new state with every planted path absent.
- **The orphan's entry point** (`orphan.txt`), on a fixture for
  `/Users/scutting/.supacode/repos/pachinko/QS02-gone-fixture`, a path that does not
  exist:
  - refused, removing nothing: with the marker missing; with the link pointing
    elsewhere; when renamed, where the check before the lock made no lock entry; with
    a machine left (naming `--delete`); for `--delete` of a name not listed; and when
    the listing fails;
  - `--delete qs02-orphan-m` deleted that machine;
  - this worktree's own state was refused as live, naming `just smolvm-purge`;
  - with everything right, it removed exactly the fixture's directory and link. The
    lock entry it left was removed by hand, and this worktree's state listing was
    unchanged.
- **The commands that run the binary,** listed by `grep -n`: the launcher's run, on the
  line right after the sentinel; the inventory; and the orphan's delete. None is in
  `scripts/install-smolvm.sh`, which only tests files and reads `codesign`.

### 10. What was written outside the worktree, and its removal

- **Step 3.** It made `~/Library/Caches/pachinko/`, `…/pachinko/smolvm/` (both 0700),
  the state directory, the state link and `/tmp/pq02-a`. Each, except the parents, was
  removed by step 3's end and again by 3b to 3d's.
- **Steps 7 and 8.** They made:
  - this worktree's state directory and state link;
  - the lock entry `~/Library/Caches/pachinko/smolvm/qs02-e906b65b.lock`;
  - the orphan fixture's directory, link and lock (all removed).

  The probe in step 8 (`logs/final2/step8-probe.txt`, 12:30 to 12:33) ran over `$HOME`, the user's
  `T/` and `C/`, `/private/tmp`, `/Library`, `/usr/local`, `/Users/Shared`, `/opt` and
  `/private/var/tmp`. Each root was listed with its own exit status and errors in
  files. It found 78 paths outside the worktree:
  - **What decision 0022 names:** the state directory's tree (52 paths) and the state
    link, and the parent's and `/private/tmp`'s entries, which changed when those were
    made.
  - **`~/Library/Caches/smolvm/vms/0d1907069827eaec/{egress,storage.raw}`:** another
    project's machine, which QS01 saw before its own smolvm ran. Its VMM is still
    running as pid 40635, and the launcher gives smolvm only the state link as its
    data root.
  - **Ordinary activity:** this session's harness and Codex logs, Docker Desktop,
    Zellij, Chrome, Tailscale's logs under `/Library/Tailscale/` and macOS temporary
    items.

  455 directories could not be read, so they are **unprobed**: `~/Library/Containers`
  and `Group Containers`, parts of `T/` and `C/`, and others. None of them names
  smolvm (`grep -c -i smolvm` printed 0, exit 1). The control `control-new` appears in
  the output. `control-old` appears too, because `touch -t` leaves the ctime new and
  the probe matches `-cnewer`, so it is no negative control.
- **After the purge:** `test ! -L` and `test ! -e` on the link and `test ! -e` on the
  directory each exited 0. The parent's names were the same before and after
  (`diff` exit 0).
- **What remains by design:**
  - `~/Library/Caches/pachinko/` and `…/smolvm/`, both 0700;
  - the lock entry `…/qs02-e906b65b.lock`, which no tool removes;
  - this session's own files under `~/.claude`, `~/.codex` and `/private/tmp/claude-501`;
  - the shared git metadata.

### 11. Checkpoint 1 record

The reviewer was Codex Sol, `codex exec -m gpt-6.2-sol -s read-only`, using AP01's
command form, over `plan.md` and `amendments.md` before any tracked change.

- **Round 1:** 34 findings (C01 to C19, B01 to B15). Triage, by a fresh subagent:
  - 15 valid, 4 valid in part, 11 duplicates;
  - invalid parts: B02's partial template, B04's short-lived children, B03's AP02 gap.

  Twelve questions went to the ruling. The evidence findings (C02, C04, C05, B11, B12,
  B14) were rerun as 3b and 3c.
- **Round 2,** over the fix diff: 22 findings, of which 17 were valid and 5
  duplicates. Two went to the ruling, as Q13 and Q14. The lead's directions:
  - no tool removes a lock entry;
  - the orphan delete becomes `--delete`;
  - `PATH` is fixed on the first line;
  - no symbolic link anywhere under the state.

The verdicts are in `verdicts.md` and `verdicts-2.md`.

### 12. Checkpoint 2 record

The reviewers were Codex Sol, over the full diff from the base with the untracked
files, and the `code-review` skill at high effort. Sol raised 27 findings (B01 to B14,
C01 to C13) and the skill 10 (R1 to R10). The lead triaged them all in
`ai_tmp/review/QS02-2/fixes.md`, and a fresh subagent applied fixes 1 to 24
(`applied.md`):

- R1: the scripts made 0755, which prek's `check-shebang-scripts-are-executable`
  requires.
- B03/C01, C13, R6: `--delete` empty, a leading `-`, or a word-split name.
- B04/C02, R5: the marker's fields and the id file, validated exactly.
- B05/C03: signal traps in every mode.
- B01/C04, R3: removal by renaming to `.purging-<name>-<pid>`, checked by inode.
- B02: a mount under the state is refused.
- B13, R2: the marker is written straight after the `mkdir`; an empty state is made
  again.
- B10/C05: the parents must be exactly 0700.
- R7, R8: the function guards made three-way; absolute helpers in the pin check.
- B07: symlinks refused in the pin check.
- R10: whitespace-only lines.
- B09, R9: `curl -q`; hashing with Python.
- B08: the stamp written with `O_EXCL|O_NOFOLLOW`, and a member named
  `.pachinko-stamp` refused.
- C06: the install lock's ACL.
- B06: `env -u SHELLOPTS …` on the recipe lines, which narrows the problem but does
  not close it (Open points).
- C07, C11: wording in decision 0022.
- R4: the template repair, documented.

Not changed in round 1: B11, B12 and C08 were narrowed by the tightened Verification
in section 14 (it cannot observe a silent invocation before the sentinel, which Q14's
ruling accepts); C09 and C10 concern the scratch drivers; C12 is Departure 6.

**Round 2.** B01 was high and valid, so §14 requires a second round over the fix diff.
The first record of this ticket (`ef511c9`) said no second round was needed, giving a
reason that is not §14's test; that was wrong, and this round corrects it. Codex Sol's
two lenses read `ai_tmp/review/QS02-2/fix.diff`: the exact diff of the three scripts
from their pre-fix versions, which were rebuilt from `diff.patch` (the launcher's
rebuilt copy equals the saved draft byte for byte), and the Justfile's and the pages'
diff from the base. 20 findings (B2-01 to B2-08, C2-01 to C2-12); the lead triaged them
and fixed, in `366ef21`:

- B2-03/C2-07: the marker and the id file are written whole to `<path>.new`, fsynced
  and linked into place; a state directory holding only an unlinked marker is an
  interrupted creation, made again.
- B2-04: a symlinked `.tools` in an orphan's recorded worktree is refused.
- B2-05/C2-05: the purge's listing runs in the foreground, so the trap keeps the locks
  until it ends.
- B2-07: the directory renamed for removal is `.purging-<hash>-<pid>`.
- B2-08: the installer's `checksums.txt` reader refuses a path that is absolute, holds
  `..`, or resolves outside the tree.
- C2-04: the orphan form repairs `tmp/` only after its worktree is shown gone.
- C2-06: the how-to checks a `.purging-*` directory's marker before its removal by hand.
- C2-11: section 14's wording.
- C2-12: the demo also writes under `target`.

Not fixed, by the ruling or as stated limits: B2-01/C2-01 and B2-02 (a process of the
same user that swaps or mounts something in the state between a check and the
removal; the trusted writers are the user and root, Q8; decision 0022 says so);
B2-06/C2-02 (`SHELLOPTS`, Open points); C2-08 (Departure 6 stands); C2-09 and C2-10
(the scratch drivers: the probe's `getconf` and some listings' statuses are not
asserted; the probe's per-root statuses and the listing files are what section 10
quotes). §14 allows two rounds, so what remains is in Open points.

**Evidence promised at checkpoint 1 and run only after round 1** (`extra.txt`): the
launcher under `TERM` and `kill -9`, an allowing ACL, and an orphan behind an
unsearchable parent (section 9). Every rerun the fixes touched was repeated on the code
of `366ef21` (sections 8 to 10, `logs/final2/`).

### 13. Departures

1. **Claim 4's evidence.** It is not the kernel's log, which did not record
   smolvm-bin's refusals (section 4); the maintainer accepted the narrower claim (Q1).
2. **The state directory's mode.** It is not 0700: smolvm sets its data root to 0755
   on first use. It is held to "not writable by group or others", and its parents
   must be exactly 0700.
3. **Profiles.** `wt-3b.sb` allowed the common git directory, so that refusal came
   from the mount. The `cmp` and `test ! -e` checks show nothing was written there.
4. **ACLs (Q8).** macOS sets deny-only entries (`group:everyone deny delete`) on `~`,
   `~/Library` and `~/Library/Caches`, so the guard refuses only an entry that
   allows access.
5. **Step 6's host check.** It ran on a copy of the installer at a fake root with a
   stubbed `/usr/bin/uname`, not with a `uname` stub on `PATH`, which the fixed `PATH`
   makes impossible (Q12 (a)).
6. **Three pages beyond Files touched,** one stale sentence each:
   `docs/reference/testing.md`, `docs/reference/agent-contract.md` (the authority
   rule's exception) and `docs/operations/maintenance.md` (the pin routine).
7. **Step 7's demo** renamed `ai_tmp` as well as the direct parent `ai_tmp/qs02` (both
   refused).
8. **The purge's stamp case.** It shows `--check` exit 0 after the stamp was restored,
   not a `cmp` of the stamp.
9. **Verification is stricter than the ticket's script.** Stdout and stderr are apart,
   stdout is `cmp`-equal to the version line, and the parent's listing and the lock's
   entry are checked (B11, B12). It still cannot show that no smolvm ran silently
   before a refusal (Q14).

### 14. Verification

`ai_tmp/qs02/verify.sh`, the ticket's script with Departure 9 applied (stdout and
stderr kept apart, the version line `cmp`-equal, the parent's names and the lock's
entry checked), run as one `sh -x` file; full output in
`ai_tmp/qs02/logs/final2/verify.txt`. Values: `version_line='smolvm 1.37.4'`,
`reached_line='smolvm launcher: running the stamped smolvm'`,
`state_dir="$HOME/Library/Caches/pachinko/smolvm/qs02-e906b65b"`,
`state_link='/tmp/pachinko-smolvm-e906b65b'`. At
`366ef2164b7624d83f8228972c12bf3b16c240f9`:

```text
366ef2164b7624d83f8228972c12bf3b16c240f9
install-smolvm: smolvm 1.37.4 is installed and matches its stamp; nothing to do.
smolvm launcher: running the stamped smolvm
tools/smolvm/smolvm.txt: sha256: '0' is not 64 lowercase hex digits
install-smolvm --check: refused: smolvm 1.37.4 is not installed as tools/smolvm/smolvm.txt pins it (no stamp at .tools/smolvm/.pachinko-stamp); run just install-smolvm
smolvm launcher: refused: smolvm is not installed as tools/smolvm/smolvm.txt pins it; run just install-smolvm
Validated 50 pages and 51 canonical topics.
Validated AGENTS.md, 2 adapters, and 14 skills.
All checks passed and the worktree is unchanged.
verify exit 0
```

The refusal's standard output was empty and its standard error held no sentinel; the
purge left no state link and no state directory, the lock's entry stayed, and the
parent's names equalled those before the run (`cmp` exit 0); `git status --short` was
empty. The same script passed earlier on `58a0f1b` (`logs/final/verify.txt`).

### 15. Open points

- **`SHELLOPTS=noexec` silently skips every recipe.** `just`'s own recipe shell
  (`Justfile` line 2, `set shell := ["sh", "-eu", "-c"]`) reads an exported
  `SHELLOPTS` before any recipe line runs. With `SHELLOPTS=noexec` exported,
  `just smolvm --version` exits 0 without running the script (launcher test case 43),
  and the same holds for every recipe, `just check` included. This is pre-existing
  and not this ticket's. A fix would change `set shell`, for example to
  `["/usr/bin/env", "-u", "SHELLOPTS", "-u", "BASH_ENV", "sh", "-eu", "-c"]`, which
  touches every gate: the maintainer's to rule on.
- **smolvm on PyPI.** It also ships as `smolmachines` (QS01 Open point 10). No ruling
  weighed it, and decision 0022 says so.
- **CI's pin check.** It runs on `ubuntu-latest` with `/usr/bin` helpers, expected
  under merged `/usr`, and was run here under `/bin/dash` (section 8) but not on a
  Linux runner.
- **A same-user swap between a check and the removal** (B2-01, B2-02): a process of
  the same user that moves the renamed `.purging-*` directory, or mounts something in
  the state, after the last check and before `rm -rf`, is not stopped. The trusted
  writers of the state are the user and root (Q8).
- **The purge's listing under a signal** was not run; `TERM` was run against the
  launcher's `run` mode only (section 9). The listing now runs in the foreground under
  the same trap.
- **Fix 4's inode-mismatch branch and fix 5's mount refusal** were not run on a real
  state directory. The helpers were tested on scratch directories, and on
  `/System/Volumes` for fix 5.
