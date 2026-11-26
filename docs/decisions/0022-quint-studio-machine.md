---
title: "Decision 0022: A project-managed machine for Quint Studio"
kind: "decision"
audience: [maintainer, agent]
canonical_for: [decision_quint_studio_machine]
requires: []
---

# Decision 0022: A project-managed machine for Quint Studio

## Context

[Decision 0021](0021-project-managed-quint-cli.md) left Quint Studio, Informal Systems'
desktop application for Quint, outside the repository: upstream ships it through a
curl-pipe installer that fetches the unpinned latest release with no checksum, installs
it system-wide and expects an account sign-in. Studio also runs headless, as
`quint-studio serve`, an embedded server. A spike on 2026-11-25 ran that server by hand
inside a smolvm guest, an x86_64 Linux machine under Rosetta on macOS on arm64, and the
maintainer ruled on its findings. The tickets that bring Studio under the repository,
QS01 to QS06, live under `.scratch/quint-studio/`; its `README.md`, "The maintainer's
rulings", states the rulings in full, and this record cites them by number. QS01's
hand-back, `QS01-studio-spike.md` there, holds the claims cited below. What this record
describes as built is QS02's, which writes it: the smolvm pin and its check, the
installer, the launcher, the purge and its orphan form. The guest's access, Studio's pin
and the other rulings below are policy that QS03 to QS05 implement; until they land, no
recipe installs or runs Studio, mounts the worktree or the common git directory into a
guest, or hides anything from one.

smolvm runs microVMs on Hypervisor.framework. Its releases are tarballs on GitHub, each
listed in the release's `checksums.sha256`. It is on neither conda-forge nor crates.io
(QS01 claim 14), so neither `pixi.lock` nor `tools.txt`
([decision 0011](0011-tool-manager.md)) can hold it. The release runs as a bash script,
`smolvm`, which points the guest agent's root filesystem and the library path at the
release's own files and executes `smolvm-bin`, a Mach-O binary whose signature carries
the `com.apple.security.hypervisor` entitlement. That signature is ad hoc, not a
Developer ID (QS01 claim 1).

smolvm keeps its machine state, a database, a usage journal, image caches, cached
registry tokens and every machine's disks and sockets, under the user's home directory
by default, shared with every other use of smolvm on the machine. `SMOLVM_DATA_DIR`
moves all of it under one root, except three writes: two templates smolvm expands
beside its own binary on first use, libkrun's eventfd files under `TMPDIR`, and an alias
under `/tmp` when a socket path is too long. A socket path is the root as written, 44
bytes and the socket's name, at most 103 bytes in all, so the root as written may be at
most 42 bytes. Kept in the worktree, under a relative root as the spike ran it, that
state would be readable by the guest, registry tokens included, unless hidden by a mount
of its own against smolvm's limit on mounts, and every command would have to run from
the worktree's root (QS01's Open points 3, 4 and 14). Kept anywhere else, it is state
outside the worktree, which is why it needs a record.

## Decision

### smolvm, pinned and installed

`tools/smolvm/smolvm.txt` pins smolvm: `version=`, `asset=`, the darwin-arm64 tarball of
that version, and `sha256=`, the archive's SHA-256, equal to its line in the release's
`checksums.sha256`. The scripts read it with `sed` and never source it. Gate 2,
`just lock-check`, runs `scripts/check-smolvm-pin.sh`, which reads only that file and
fails, offline, unless it holds exactly those three keys in their form, printing
`tools/smolvm/smolvm.txt: <key>: <reason>`. No Renovate manager reads it; it is moved by
hand.

`just install-smolvm` runs `scripts/install-smolvm.sh`. It refuses any host but macOS on
arm64, a pin that fails the gate's check, and a symbolic link or anything but a
directory at `.tools` or `.tools/smolvm`. It takes the install lock, `.tools/smolvm.lock`,
and refuses at once if another process holds it. It skips, with no network, when the
stamped install passes the skip test below. Otherwise it downloads the tarball over
HTTPS alone into a new temporary directory under `.tools/`, refuses it unless its
SHA-256 is the pin's, checks every member's name, unpacks it with the archive's modes,
compares the unpacked tree entry by entry with the archive's members, checks the files
the release's own `checksums.txt` lists, checks `smolvm-bin` with
`codesign --verify --strict` and reads the entitlement from its signature, writes the
stamp, and only then moves the tree into `.tools/smolvm/`. It computes both SHA-256
checks with the pixi Python's `hashlib`, not `/usr/bin/shasum`, a Perl script that reads
`PERL5OPT`, and runs `curl -q`, which reads no `~/.curlrc`; it refuses an archive member
named `.pachinko-stamp` and makes the stamp new without following a link. It never runs
the binary, never signs it or removes its signature, and never touches an extended
attribute.
`just initialize` runs it after `just install-apalache` on macOS on arm64, and on any
other host prints that smolvm and Studio are skipped and goes on.

The stamp, `.tools/smolvm/.pachinko-stamp`, records `archive=`, the pinned hash, and
`tree=`, a digest over one record for every entry of the tree: its type, its mode, its
content's SHA-256 or a symbolic link's target, and its path. The digest leaves out the
stamp and exactly two top-level files, `overlay-default.ext4` and
`storage-template.ext4`, the templates smolvm expands beside its binary on first use, so
a first boot leaves the stamp good. `scripts/install-smolvm.sh --check`, offline and
read-only, applies the same skip test: the stamp, the digest recomputed now, `smolvm` and
`smolvm-bin` executable, and both `codesign` reads, on every call.

### The launcher

`scripts/smolvm.sh` is the one way smolvm runs here. `just smolvm <args>` runs it in its
`run` mode and `just smolvm-purge` in its `purge` mode, and the later tickets' scripts
call it directly, without `just`, as `/bin/sh <path>/scripts/smolvm.sh` or as the
executable it is. The recipes, like `just install-smolvm` and `lock-check`'s pin line,
start it as `/usr/bin/env -u SHELLOPTS -u BASH_ENV -u ENV -u PS4 /bin/sh`, so none of
those can make the script's shell skip or trace it; the recipe's own `sh` has read them
already, which that does not undo. In one shell process, `run`:

1. fixes `PATH`, refuses an exported shell function, and names every helper by its
   absolute path;
2. takes the install lock, then runs `--check`, so no install replaces the tree between
   the check and the run;
3. refuses a caller's `SMOLVM_*`, `DYLD_*`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`,
   `XDG_CONFIG_HOME` or `DOCKER_CONFIG`, a `HOME` other than the user database's, and a
   `--state-dir` before any `--`, since each would move smolvm's state or change what it
   loads;
4. takes the state lock, and under both locks checks, or makes, the state below;
5. prints `smolvm launcher: running the stamped smolvm` on standard error and runs
   `.tools/smolvm/smolvm` by path, with a cleared environment that holds the fixed
   `PATH`, `TERM`, `HOME` and `SMOLVM_DATA_DIR` set to the state link, `TMPDIR` to the
   state's `tmp/` and `DOCKER_CONFIG` to its empty `docker-config/`, reached through the
   link as it is written.

Everything the launcher prints goes to standard error, so smolvm's standard output is
smolvm's alone. It holds both locks until smolvm ends and closes their descriptors for
smolvm, so a machine that goes on running after its command returns holds neither. Every
mode traps HUP, INT and TERM before its first lock and waits for any smolvm it starts;
only `kill -9` of the launcher alone frees the locks while smolvm still runs. A second smolvm command for the same worktree, or
its purge, is refused while the locks are held, naming the process that holds them; a
reinstall is refused too, naming what kinds of command can hold the install lock but
not the process. A command that lasts a session, as QS05's will, holds both locks for
its whole run.

### Where the state lives, and what removes it

Ruling 5 puts the machine's state outside the worktree, in a durable directory of its
own for each worktree, reached through a short link in `/tmp`. Each path:

- **The state directory**, `~/Library/Caches/pachinko/smolvm/<worktree>-<hash>/`.
  `<worktree>` is the worktree's directory name lowercased, with every character outside
  `a-z`, `0-9` and `-` replaced by `-`; `<hash>` is the first eight hex digits of the
  SHA-256 of the worktree's physical path. Its parents below the home directory are
  made with mode 0700 when absent, and each, from the home directory down, must be a
  real directory owned by the user, writable by neither group nor others, with no ACL
  entry that allows access; the two the launcher owns, `~/Library/Caches/pachinko` and
  its `smolvm/`, must have mode 0700 exactly, and any other mode is refused, naming the
  `chmod 700` that mends it. The state directory is made 0700; smolvm sets it to 0755 on
  first use, so the launcher holds it to the parents' rule, and its parent, 0700, keeps
  everyone else out. It holds the marker `.pachinko-state`, written at once after the
  directory is made, recording the worktree's physical path (`path=`, absolute), a
  random id (`id=`, 32 lowercase hex digits) and the pin's hash (`pin=`, 64), each once
  and nothing else; `tmp/` and `docker-config/`, each 0700, the second kept empty and
  either made again when missing; and smolvm's own `Library/` tree, with its database,
  usage journal, image caches, registry tokens and machines. A state directory that is
  wholly empty, left by a creation interrupted before its marker, is removed and, by
  `run`, made again. A symbolic link anywhere under it is refused, since smolvm would
  write through it.
- **The worktree's id file**, `.tools/smolvm-state-id`, inside the worktree and ignored.
  State is reused only when the marker's `path=` is this worktree's physical path and
  its `id=` is the id file's, so two worktrees whose hashes collide never share state,
  and a worktree made again at an old path does not adopt the old one's.
- **The state link**, `/tmp/pachinko-smolvm-<hash>`, 29 bytes, within smolvm's 42. It
  must be the user's own symbolic link to exactly the state directory; anything else
  at that name is refused and never followed. macOS empties `/tmp` on a restart, and
  the launcher makes the link again.
- **The state lock**, `~/Library/Caches/pachinko/smolvm/<worktree>-<hash>.lock`, beside
  the state directory, and **the install lock**, `.tools/smolvm.lock`, inside the
  worktree. Each is a regular file locked with `lockf`; no tool ever removes either
  entry, because two callers that found and missed a removed entry would hold two
  locks. A user may delete a stale one by hand while no pachinko smolvm command runs.

`just smolvm-purge` removes the state link and the state directory and nothing else.
Like the launcher, it makes the parents and the state lock's entry when they are
missing, and its orphan form makes an orphan's state link again before it removes it.
It needs the stamped install, takes both locks, checks the state as the launcher does,
and takes smolvm's own listing of the machines there; it refuses, removing nothing,
when the listing fails or lists any machine, naming each and how to delete it. It then
walks the state directory and refuses a mount point under it, renames the directory to
`~/Library/Caches/pachinko/smolvm/.purging-<name>-<pid>` beside it, checks that what it
renamed is the directory it checked (the same device and inode, still holding the
marker), removes the state link, and removes the renamed directory. A removal that fails
part way leaves only that `.purging-*` directory, which nothing uses and which is
removed by hand. With no state it says so, removing a state link left pointing at the
absent directory. It keeps the lock entry, the parents and the id file.
`just smolvm-purge --orphan <name>` does
the same for the state of a worktree that is gone, `<name>` being the state directory's
name: the marker's recorded worktree must no longer exist or no longer hold the
marker's id, and the state directory's name must be the one that path forms.
`--orphan <name> --delete <machine>` deletes one machine that the orphan's listing has
just listed, matched exactly; an empty name or one starting with `-` is refused. Studio's
data with its sign-in is not this state: QS04 proposes where it
lives and what removes it, and purging this state does not remove that.

A state directory whose marker records another pin is accepted for `machine ls`,
`machine stop` and `machine delete` alone, and by the purge, because an old state can
supply a new release with an old release's expanded templates, from its `.smolvm/` and
`Library/Caches/smolvm/`, and with an old database; the templates are read from smolvm's
source, and the upgrade was not run. Whoever moves the pin purges first,
with the old install in place; a worktree that receives a moved pin installs it, deletes
its machines and purges. [Maintain dependencies](../how-to/maintain-dependencies.md#moving-the-smolvm-pin)
gives both orders.

### The guest's access

This section is policy for QS04, which implements it. Ruling 2, as amended: the worktree
is mounted read-only, with a few writable subdirectories that QS04 names; the common git
directory is mounted read-only (ruling 7); the guest has open egress, with host loopback
and private ranges blocked. Where in the guest the worktree is mounted is not part of
the ruling: QS02 mounted it at its own host path, and QS04 proposes the same. The
read-only mount, the writable subdirectories and the network flags are arguments given
to smolvm, not something the launcher adds. So the boundary below holds for the
configuration QS04 runs, and for smolvm's own state, which the launcher fixes.
`just smolvm` passes its arguments through, and what a developer mounts by hand with it
is theirs.

What the guest cannot write: anything in the worktree outside the writable
subdirectories. QS02 showed it from the guest in two runs, which its hand-back records.
By hand, at its step 3, writes to the common git directory and to `target/` were
refused. Through `just smolvm`, at its step 7, a write in a writable subdirectory nested
in the read-only worktree succeeded and the host saw it, while writes to the tracked
`Justfile`, to the worktree's `.git` and to `.tools/`, and renames of the writable
subdirectory and of its parent, were refused, the host's copies unchanged. So the guest
cannot change the `Justfile`, the scripts, or any ignored tree the host runs or loads.
What it writes in a writable subdirectory is tracked or ignored as QS04 rules, and for a
tracked file the control is the maintainer reading the diff.

What the guest can read, and send: the whole worktree, except what ruling 6 hides, which
QS04 lists and the maintainer rules on, and the common git directory, which is the
primary checkout's whole `.git`. Its egress is open, so what it can read it can send
out. Published ports bind `127.0.0.1` and `[::1]` by smolvm's default, which the
launcher keeps by refusing `SMOLVM_PUBLISH_ADDR` with every other `SMOLVM_*` variable.
By smolvm's source, turning a machine's networking off and on again clears its
private-range block and its deny rules; nothing here does that.

### Studio's pin, and what it does not cover

Ruling 1: QS03 pins Studio's AppImage by its observed SHA-256, and a new upstream build
fails the install until it is pinned again. That detects drift; it does not make
installs reproducible. Ruling 9: Studio is not hermetic, and the pin covers none of
what it does at run time. QS01's Open point 7 records that it downloads an executable
into its data directory, has an `upgrade` command, checks for updates, queues telemetry
and carries a crash-report client. Egress stays as ruling 2 has it.

### The other rulings, and who implements them

- Ruling 3: wiring agents to Studio's MCP endpoint is QS05's, which adds here the route
  it chose and what a caller can reach through it.
- Ruling 6: hiding a project-managed ignored tree from the guest under an empty
  directory mounted over it, QS04, which proposes the list for the maintainer's ruling.
  That such trees share one parent, so one mount hides them all, is the lead's reading
  of the maintainer's answer, not its words.
- Ruling 7: the wrapper around the guest's `git` that sends the objects Studio adds to
  the machine's own disk, comparing the common directory with the exact mounted path
  before it redirects, QS04.
- Ruling 8: a host port searched for each worktree, QS04; the host port is never 7000,
  whether searched for or set by hand.
- Ruling 9: what the pin does not cover, here and in QS03.
- Ruling 10: the guest's packages from a dated `snapshot.debian.org` timestamp, QS04,
  with QS06 to find out whether the guest image can be built and hosted on GitHub.

QS03 and QS04 add their own particulars to this record when they land.

### The supported host

Ruling 4: macOS on arm64 is the only supported host, and Linux is recorded as untested.
The installer refuses any other host, and `just initialize` skips smolvm there. CI never
installs smolvm or boots a machine, since no runner offers the hypervisor, and
`just check` runs neither smolvm nor its installer.

## Consequences

Machine state now lives outside the worktree, and deleting a worktree leaves it behind.
`just smolvm-purge` removes it from its own worktree, and `--orphan` removes it once the
worktree is gone; each marker names its worktree, so an orphan can be traced.
[Develop locally](../how-to/develop-locally.md#smolvm) says how. Lock entries and the
parents stay for good unless removed by hand. Even `just smolvm --version` makes the
state. smolvm's sparse templates beside the binary, of 10 GB and 20 GB apparent size,
and its image caches in the state, take disk.

The stamp follows Apalache's model. It detects drift and damage in a tree the installer
wrote; it does not detect a stamp rewritten by hand to match a changed tree. It does not
vouch for the two first-use templates it leaves out, and smolvm uses a plain template at
either name as it finds it, so every install starts from a new, empty directory and
never carries old ones. The repair for a damaged template is to remove
`.tools/smolvm/overlay-default.ext4` and `storage-template.ext4`: smolvm expands them
again on its next use, and `--check` still passes. The ad hoc signature shows that `smolvm-bin` is intact against
its own embedded hashes and says nothing of who built it; the release's checksum is the
only provenance. Every `--check`, so every `just smolvm`, runs both `codesign` reads.
macOS's `/usr/bin/tar`, which the installer runs, ignores `TAR_OPTIONS`; the installer
unsets it and its BSD and GNU siblings anyway.

Unlike Apalache's, this install is locked against its users: a reinstall is refused
while any smolvm command of the worktree runs, and a command is refused while a
reinstall runs. The cost is that a command lasting a session blocks every other smolvm
command for that worktree, its purge and its reinstall, until it ends; the launcher's
and the purge's refusals name the process to end.

The evidence that the state link is used as it is written is narrower than first asked
for. The kernel's sandbox log did not record smolvm-bin's refusals on this host, so the
evidence is smolvm's own error and the relink: under a sandbox profile that refused
every new name in `/tmp`, a 60-byte root failed at the alias for its agent socket,
named by smolvm's own error, while roots of 11 and 29 bytes booted, served and listed,
and a machine was found again, with a file its guest wrote before the stop, after the
link was removed and made again.
So a state link within the limit makes no fatal alias attempt. That smolvm never
attempts an alias at all is not shown.

By its source, not run, smolvm does not refuse to make a machine on a host without
Rosetta (QS01 claim 3), and the launcher does not test for it: whatever makes a machine tests first, as QS04's `up` will. smolvm
keeps its usage journal, on by default and uploaded only when its configuration says
so, inside the state directory, and the purge removes it with the rest.

The guest reads the worktree and the primary checkout's `.git`, and can send what it
reads anywhere egress reaches. That is accepted under ruling 2; ruling 6's hiding is the
only narrowing, and QS04 proposes what it covers.

## Alternatives

- **State inside the worktree**, as the spike ran it with a relative root: the guest
  would see it, registry tokens included, unless hidden by a mount of its own, against
  smolvm's limit on mounts, and every command would have to run from the worktree's
  root. Ruling 5 took it out.
- **A plain directory in `/tmp`**: short enough, but a restart empties it.
- **`--state-dir`**: smolvm always makes it absolute, so a root inside a worktree
  overruns the limit.
- **pixi or `tools.txt`**: neither can hold smolvm (QS01 claim 14). smolvm also ships on
  PyPI as `smolmachines`, which claim 14 did not cover and this record has not weighed.
- **The launcher in the recipe's body**: the later tickets call it without `just`, and
  their callers read smolvm's standard output alone.
- **Waiting for a busy lock**: a busy lock is refused at once instead, naming its holder
  where it can.
- **Removing a lock entry** after use: two callers that found and missed a removed entry
  would hold two locks.
- **The tree's digest committed in the pin file**, so a hand-written stamp would fail:
  it changes the pin's three keys and every bump, and was not taken.
- **The installer expanding both templates itself**, so the digest covers them: it needs
  a zstd decompressor macOS does not ship, a new dependency.
- **The launcher refusing a mount outside the worktree**: it ties the launcher to
  smolvm's argument syntax and still allows a writable mount of the whole worktree.

## What would reopen this

smolvm in a registry pixi or cargo-binstall can install from with a checksum the
lockfile holds, which would retire the installer and the pin file. A Developer ID
signature on its releases, which would give the binary a provenance beyond the release
checksum. A smolvm whose socket paths no longer limit the root, which would retire the
state link. A CI runner that offers the hypervisor, or a supported Linux host. Upstream
publishing Studio itself with a checksum, which would reopen QS03's pin.

## Related pages

- [Decision 0021: A project-managed Quint CLI](0021-project-managed-quint-cli.md)
- [Decision 0011: Tool manager](0011-tool-manager.md)
- [Develop locally](../how-to/develop-locally.md)
- [Maintain dependencies](../how-to/maintain-dependencies.md)
- [Security model](../explanation/security-model.md)
