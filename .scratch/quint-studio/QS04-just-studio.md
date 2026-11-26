---
id: QS04
title: "just studio: Quint Studio running in the project's machine"
status: todo
depends_on: [QS02, QS03]
parallel_with: []
branch: ticket/qs04-just-studio
estimated_size: M
---

# QS04: `just studio`: Quint Studio running in the project's machine

## Context

After QS02, `just smolvm` runs a smolvm install this repository pins and checks, with its
machine state where decision 0022, `docs/decisions/0022-quint-studio-machine.md`, says:
outside the worktree. After QS03, `just install-quint-studio` installs the Studio
AppImage whose SHA-256 `tools/quint-studio/studio.txt` records. Nothing yet puts the two
together. QS01, `.scratch/quint-studio/QS01-studio-spike.md`, did that by hand, with
flags typed at a shell and the worktree mounted read-write; this ticket turns its
hand-back, and the rulings given after it, into a committed Smolfile and one recipe.

The maintainer's rulings, listed in the README of these tickets,
`.scratch/quint-studio/README.md`, bind this ticket. Ruling 2 was amended and rulings 5
to 10 were given after QS01's hand-back, so several of them were never run. The ones
that shape this ticket:

- **Ruling 2, as amended,** sets the guest's access: the worktree mounted read-only,
  with a few writable subdirectories that this ticket names; open egress; host loopback
  and private ranges blocked. Mounting it at its own path is how QS01 ran it and what
  this ticket proposes, not part of the ruling. QS01 ran Studio only over a read-write
  worktree, so Studio under this mount is untested, and step 3 proves it before
  anything is built on it.
- **Ruling 4** limits it to macOS on arm64.
- **Ruling 5** puts smolvm's machine state outside the worktree. QS02 built it: **the
  state directory**, a durable directory for each worktree, and **the state link**, the
  short symbolic link smolvm is given as its state root. This ticket uses both as
  QS02's hand-back names them and makes neither.
- **Ruling 6**, which the README gives as the lead's reading of the maintainer's answer
  "One hidden parent" and not as the maintainer's words: the read-only mount stops
  writes, not reads. A project-managed ignored tree that the guest must not read is
  hidden by an empty directory mounted over it, and such trees share one parent, so
  that one mount hides them. Which paths are hidden is this ticket's list, and the
  maintainer rules on it at step 5.
- **Ruling 7:** the common git directory is mounted read-only, and this ticket ships a
  wrapper around the guest's `git` that sends the objects Studio adds to a directory on
  the machine's own disk, after comparing the common directory with the exact mounted
  path.
- **Ruling 8:** the UI is never published on host port 7000. `just studio up` searches
  for a free host port for each worktree and records it, and the URL it prints is built
  from that record. QS05 is not a reader of the UI's port.
- **Ruling 9:** Studio is not hermetic, which is accepted and documented. The page this
  ticket writes says so, citing decision 0022.
- **Ruling 10:** the guest's packages come from a fixed `snapshot.debian.org` timestamp,
  install-tested here. Building a guest image is QS06's, later.

Three of QS01's findings change the design beyond the rulings, and each is cited where
it binds: a volume on the machine's disk is deleted with its machine (claim 16), the
AppImage prints no version (claim 12), and a machine carries a limited number of mounts
(claim 16). This ticket states what must hold and names the claim or the "Facts for"
section of QS01's hand-back that holds the particulars; step 1 reads them there.

Studio's MCP endpoint is QS05's. Nothing here connects to it.

Read these:

- `.scratch/quint-studio/README.md`, "The maintainer's rulings".
- `.scratch/quint-studio/QS01-studio-spike.md`, its hand-back notes: "Facts for QS04";
  the entry on a stale extraction in "Facts for QS03"; the entries on a Rosetta
  preflight and on a machine's network settings in "Facts for QS02"; the claims on the
  UI's answers, extraction, sign-in, git, persistence, egress, the version, masking and
  machine control (claims 4 to 9, 12, 13 and 16); "Go or no-go", for the three ways it
  lists; Open points 7 and 14 to 16; and "Rulings after the hand-back".
- `.scratch/quint-studio/QS02-pinned-smolvm.md` and
  `.scratch/quint-studio/QS03-pinned-studio.md`, their hand-back notes, for the rulings,
  the state directory with its marker, the state link, the lock, everything the
  launcher writes outside the worktree, the removal recipe with what it leaves behind,
  and the `--check` modes this recipe calls.
- `docs/decisions/0022-quint-studio-machine.md`.
- `Justfile`: `smolvm`, `install-smolvm`, `install-quint-studio`, `quint`.
- `docs/how-to/develop-locally.md`, "Optional: Quint Studio".
- `.agents/skills/review-docs/SKILL.md`, for the new page.
- `tickets/CONVENTIONS.md` §11, §13 and §14.

## Goal

`just studio up` boots Quint Studio's `serve` inside a machine built from the committed
Smolfile, with the worktree mounted read-only but for its writable subdirectories, and
prints the UI's token URL on host loopback. `just studio status`, `stop` and `down`
report, stop and remove the machine. The recipe runs only over the stamped smolvm and
Studio installs, publishes the UI on a free host port it searches for, refuses an
explicit `STUDIO_UI_PORT` that is taken, and keeps Studio's sign-in and projects in a
directory outside the machine, which survives `stop`, `down` and `up`. A new how-to
page, "Run Quint Studio", is the one place a contributor learns to use it.

The demo: `just studio up`; the maintainer signs in; Studio opens the worktree as a
project and runs one scan; after `just studio stop` and `just studio up`, and again
after `just studio down` and `just studio up`, it is still signed in with its projects
intact.

## Design proposed in this ticket

These are proposals. Step 1 amends them where QS01's hand-back or the QS02 and QS03
rulings disagree, step 3 proves by hand the parts that were never run, and the
maintainer rules on the result at step 5, with the proof and checkpoint 2's findings in
hand.

- **A committed Smolfile, `tools/quint-studio/Smolfile`**, holding what every worktree
  shares, in the keys upstream documents (QS01 "Facts for QS04"). QS01 made its
  machines with flags and ran none of those keys, as that section says, so the Smolfile
  is itself unproved: step 3 makes its machine from the draft, in the form `up` will
  use. It holds:
  - the guest image by the digest "Facts for QS04" says to pin, the one that gives the
    guest the right architecture, never by tag alone;
  - Rosetta on, with the CPU count and memory QS01 claim 11 found usable;
  - networking on, with private ranges and `127.0.0.0/8` denied, plus whatever QS01
    claim 9 found DNS needs. Claim 9's control ran in one address family, so step 3
    finds every family in which the guest has a route to the host, and the denial
    covers each one it finds;
  - the guest path of Studio's data, where the data directory (below) is mounted;
  - the stamped `.tools/quint-studio/` read-only at a fixed guest path;
  - no workload that runs `serve`. A machine made from the Smolfile starts with nothing
    serving, on its first start and on every later one. `up` runs
    `serve --bind 0.0.0.0` itself, by one guest command, the way QS01 claim 4 and
    "Facts for QS04" found Studio runs in the guest and claim 16 found a long-lived
    command is started in a running machine, and only after the checks below.

  What differs per worktree, the worktree's own path, the writable subdirectories, the
  hidden paths, the common git directory, the data directory and the host port,
  `scripts/studio.sh` passes as flags, if step 2 finds smolvm accepts flags beside a
  Smolfile. Otherwise it renders a copy of the Smolfile into the state directory, where
  QS01 claim 16 found smolvm leaves a host file alone, with those values filled in.
  "Facts for QS04" records how a Smolfile expands a variable under a state root like
  QS02's, and what a missing host path does to the create; step 1 reads both before the
  plan relies on either.
- **Guest packages** (ruling 10). The image is the distribution's own and lacks what
  Studio and its git commands need; "Facts for QS04" lists the packages. A provisioning
  step, run on a machine `up` has just made, replaces the image's package sources with
  one fixed `snapshot.debian.org` timestamp, recorded with the Smolfile, refreshes the
  indexes and installs from it. It records the resolved version and the origin of every
  package it installed. There is no fallback to a live mirror: a failed install stops
  `up`, and `up` never starts `serve` on a machine whose provisioning did not finish.
  An unpinned `apt-get install` is not a choice: it would install whatever the mirror
  holds that day. Whether the provisioning runs as a first-start command or as a command
  `up` runs in the guest is the plan's, from QS01 claim 16; either way `up` has its exit
  status before it goes on, and it ends by writing the completion record (below). A
  guest image that already carries the packages, pinned by digest, is QS06's, after the
  first working version.
- **The mounts** (ruling 2):
  - the worktree, read-only, at its own host path, so that paths in Studio, in git and
    in an agent on the host agree;
  - each writable subdirectory, read-write, at its own path inside it. `quint-specs/`
    is the ruling's example, not a requirement: step 1 proposes the list from what QS01
    found Studio writes in a worktree (claim 8 and "Facts for QS04"), step 3 adds any
    path Studio needed and could not write, and the maintainer rules on the list at
    step 5;
  - the common git directory, read-only, at its own host path (ruling 7): a guest that
    could write it could plant hooks or configuration the host's git runs;
  - the Studio install, read-only;
  - the data directory (below), read-write;
  - an empty directory over each hidden path (below).

  Every mount counts against the limit QS01 claim 16 found, so
  `ai_tmp/qs04/amendments.md` carries a table of every mount: host path, guest path,
  mode, why it is there, and what it costs by that claim. The layout has a budget: the
  limit claim 16 found for the network settings this design uses, charging everything
  that claim says counts. `scripts/studio.sh` counts the same way and refuses a layout
  over the budget. A layout that does not fit is a **stop** for the maintainer at step
  5; the builder drops no mount to make it fit.

  Before it creates or mounts anything, `scripts/studio.sh` refuses, exiting 1 and
  naming the recovery, a symlink or anything but a directory at each hidden path, each
  writable subdirectory, each of their ancestors below the worktree root, and each host
  directory outside the worktree that a mount needs, as QS02's installer does for
  `.tools`. Only then, and before the boot, does it make a host directory a mount needs
  that is absent, so that the creation cannot follow a link out of the worktree and the
  create does not fail on a missing path.
- **The git wrapper** (ruling 7), a committed script the provisioning step installs in
  the guest in front of the real `git`. "Facts for QS04", under "The git mount that
  works", says what the wrapper QS01 tested did, where it fell short, and what else git
  needed in the guest; the design is that wrapper with the ruling's change. It redirects
  only when the repository's common directory is exactly the mounted path, compared as
  a whole path and not by its beginning. Then new objects go to a directory on the
  machine's own disk and the host's objects are still read. With any other common
  directory it redirects nothing, so a repository of the guest's own is untouched. A
  caller that sets its own object directory keeps it. Every call is passed on to the
  real `git`. Steps 3 and 8 quote the exit status of each git command a scan runs, so
  the wrapper may keep a record of each call and its status on the machine's own disk.
  Whether the committed wrapper keeps one always, only when asked, or never, with
  Studio's own logs as the source, is the plan's to propose (Open points).
- **Hidden paths** (ruling 6). The read-only mount stops every write outside the
  writable subdirectories, so nothing is hidden to protect it from a write. A path is
  hidden so that the guest cannot read it: the guest has open egress, so what it can
  read it can send out. At step 1 the builder lists every pattern in `.gitignore` in
  `ai_tmp/qs04/amendments.md`, each with whether the host runs or loads what lies there
  and whether the guest may read it. What may hold a credential or private data is
  proposed hidden: `ai_tmp/`, where reviews, proofs and verification logs are kept, and
  the ignored local settings under `.claude/` and `.codex/`, among them. The plan
  resolves what the guest can actually read, not only what the patterns name: every
  ignored path that exists, an ignored file at the worktree root included, and every
  path a second mount shows elsewhere, such as the common git directory. QS01 claim 13
  found that a single file cannot be mounted over, so hiding one file means hiding its
  directory, tracked files in it included, and a file at the worktree root cannot be
  hidden without hiding the whole mount. A path that must be hidden and cannot be is a
  **stop** for the maintainer.

  Each hidden path gets an empty directory mounted over it, read-only as QS01 claim 13
  ran it. QS01 ran that only inside a read-write mount (its Open point 14), so step 3
  proves each hidden path with a harmless sentinel: present on the host, absent in the
  guest, still present on the host afterwards. `up` checks before every boot that the
  directory it mounts is empty. A new project-managed tree that the guest must not read
  goes under a parent that is already hidden, so that it costs no mount. The maintainer
  rules on the hidden list at step 5.
- **Studio's data.** By QS01 claim 8 Studio's data stayed on the machine's own disk in
  the spike, and by claim 16 anything on that disk, a volume included, is deleted with
  the machine. So a `down` that deletes the machine would delete the sign-in. The
  contract stays: `down` keeps the sign-in and the projects. The proposal is **the data
  directory**: a directory for each worktree beside the machine's state, outside the
  worktree (ruling 5), mounted read-write at the guest path of Studio's data. It is a
  mount and is counted. It holds a credential, and whatever Studio downloads into its
  data at run time (QS01 Open point 7), which ruling 9 accepts. The plan says, for every
  path QS01's claims 6 and 8 found Studio writing outside the worktree, whether it lies
  in the data directory or is lost at `down`. This is untested, so step 3 proves it. If
  it fails, the builder does not choose: **stop** for the maintainer's ruling between
  another storage design and a `down` that deletes the data and says so. The
  directory's path is the plan's to propose and step 5's to rule on (Open points).

  The data directory is bound to its worktree, as QS02 binds the state directory. With
  it `up` keeps an identity record: the worktree's physical absolute path and the id
  the worktree itself keeps for QS02's marker, as QS02's hand-back names them. Two
  things must hold. The same checkout keeps its data across `down`, QS02's removal
  recipe and the next `up`, which `down` itself points to and a move of the smolvm pin
  asks for. And another checkout at the same path does not take that data over. Step 1
  reads in QS02's hand-back which id outlives that recipe; if none does, the question
  goes to step 5's ruling (Open points). The record lies where the guest cannot write
  it, so outside the directory that is mounted, and the command `down` prints removes
  both. If the ruling puts the data inside QS02's state directory, QS02's marker is
  that record. `up` writes the record when it makes the directory, and never
  afterwards. Before `up` mounts the data directory, and before `down` prints the
  command that deletes it, the record is read without following a link. A record that
  is missing, unreadable or a symlink, or that names another path or another id, is a
  refusal: nothing is mounted, and `down` prints no deletion line for the directory.
  The message names the directory, what its record says, and the way out: the command
  that removes that directory, to be run only once the reader has seen that the
  checkout the record names is gone, since it deletes whatever sign-in lies there. A
  directory found there with no record is never adopted. So a checkout made again at an
  old path does not take over the sign-in left there, and two worktrees whose names and
  hashes collide never share one.
- **`scripts/studio.sh`, run by `just studio <action>`**, POSIX `sh`. Every smolvm
  command it runs goes through the launcher `just smolvm` runs, so QS02's checks of the
  state directory and the state link, its lock and its environment hold for every
  action, and this script makes neither the directory nor the link.
  - `up` refuses any host but macOS on arm64, and a host that fails the Rosetta
    preflight "Facts for QS02" describes, for the reason given there; runs
    `scripts/install-smolvm.sh --check` and `scripts/install-quint-studio.sh --check`,
    failing and naming `just install-smolvm` or `just install-quint-studio`; and checks
    the layout: the refusals and the budget above, and the data directory's identity
    record. It then computes the machine's input digest: the SHA-256 of the smolvm
    stamp's `archive=`, the Studio stamp's `sha256=` and `version=`, the Smolfile's
    SHA-256, the SHA-256 of the provisioning step, of the wrapper and of the recorded
    snapshot timestamp, and every per-worktree value it passes: each mount, the
    writable list, the hidden list, the data directory, the publication policy and a
    port set by hand. A searched port is not in the digest; it is recorded beside it.

    A machine that exists whose recorded digest differs, or that has none, is stopped
    first if it runs, so that nothing it serves outlives the refusal; then `up` refuses,
    naming `just studio down` and then `just studio up`, which rebuilds the machine and
    keeps the data directory. A machine already running with the same digest is
    reported, not restarted, and no search is made, since the listener is its own; but
    only after the confirmation a first start gets (Ports, below): its recorded port
    against its actual mapping, every listener on that port its own, and a 200 with the
    token through it. A record that disagrees with the mapping is never repaired: `up`
    stops the machine and refuses, naming `just studio down`. A listener there that is
    not its own means another process has taken the recorded port, which the rule for
    an existing machine covers (Ports, below). If only the answer is missing, `serve`
    is dead: `up` stops the machine and starts it as it would a stopped one.

    Otherwise `up` takes the host port (below), creates the machine if it is absent,
    recording the digest beside the machine's state where QS01 claim 16 found smolvm
    leaves it alone, and starts it. Nothing serves yet, since the Smolfile runs no
    `serve`. A machine `up` has just made is then provisioned: the packages, the
    wrapper, and the extraction of the AppImage, done in the guest the way QS01 claim 4
    and "Facts for QS03" found. By QS01 claim 12 the guest cannot read a version from
    the AppImage, so `up` reads none. As the entry on a stale extraction in "Facts for
    QS03" sets out, the extraction writes a stamp beside the extracted tree, holding the
    SHA-256 of the AppImage it came from, and writes it only after extracting from a
    copy that was hashed against the pin and could not change in between. The last act
    of provisioning, after the packages, the wrapper and the extraction have each
    exited 0, is to write **the completion record** on the machine's own disk. It is
    deleted with its machine, so no earlier machine's record can stand for a later
    one's.

    Before every `serve`, on a first start or a later one, `up` checks two things in the
    guest: that the completion record is there, and that the extraction's stamp equals
    the Studio stamp's `sha256=`. A provisioning step that did not exit 0, a missing
    record, a missing stamp or a stamp that differs stops `serve`: `up` stops the
    machine and refuses, naming `just studio down` and then `just studio up`, and never
    falls through to the tree already there. A later `up` meets the same machine and
    refuses it the same way, since it provisions only a machine it has just made.
    Whether it should instead delete such a machine is in Open points. The check runs
    at every start, so it cannot sit in a command smolvm runs on the first start only.
    Neither the extracted tree nor the completion record ever lives in the data
    directory, which outlives the machine.

    Only then does `up` start `serve`, by the one guest command, wait for the UI,
    failing closed on a timeout with the guest's last log lines, any token in them
    replaced, confirm the port (below), record it, and print the host URL and the port
    line. Every refusal exits 1 with nothing of this worktree's served when it returns,
    and a message naming what to run. The refusals over the host, the stamps, the
    layout, the data directory's identity and a port set by hand that is malformed or
    already taken come before anything boots.
  - `status`: whether the machine exists and runs, the pinned version and whether the
    extraction's stamp matches the pin, and, when it runs, the host URL and the port
    line, after the confirmation `up` makes of a running machine. If that fails,
    `status` says what failed and exits 1. It changes nothing.
  - `stop`: stops the machine and keeps it and the data directory.
  - `down`: deletes the machine, with the records kept beside its state, the digest and
    the port, and keeps the data directory. It prints the command that deletes the data
    directory with its identity record, after the check of that record above, and says
    that doing so deletes the sign-in and every Studio project; the recipe never runs
    it. It also names QS02's removal recipe for the machine's state.
  - Any other action, or none, prints the four and exits non-zero.
- **The machine's name** is `pachinko-studio-<worktree>-<hash>`: the worktree's
  directory name lowercased, every character outside `a-z`, `0-9` and `-` replaced by
  `-`, and the first eight hex digits of the SHA-256 of the worktree's absolute path.
  Each worktree has its own state directory, by QS02, and its own data directory, whose
  path is formed from the worktree the same way. Neither rests on the name alone: QS02's
  marker binds the state directory to its worktree and the identity record binds the
  data directory, so two worktrees never share a machine or Studio's data.
- **Ports** (ruling 8). The range, the override and the reuse are proposals that step
  5's ruling can change. Studio's port in the guest is its own, 7000. The host port is
  never 7000, searched or set by hand. It is published on host loopback only, in the
  address families QS01 claim 5 recorded smolvm binding, never on another host address,
  and the variable claim 5 names as widening the publication never reaches smolvm from
  the caller.
  - With `STUDIO_UI_PORT` unset, `up` searches a range that does not hold 7000,
    proposed as 7100 to 7199, for the first port on which
    `lsof -nP -iTCP:<port> -sTCP:LISTEN` finds no listener. A failure of `lsof` itself
    is a refusal, not a free port, and the script tells the two apart. A range with no
    free port is a refusal.
  - With `STUDIO_UI_PORT` set, the value must be a decimal integer from 1024 to 65535
    and not 7000, and the same test must find no listener; otherwise `up` refuses,
    naming the process and `STUDIO_UI_PORT`. A port set by hand is never searched
    around.
  - A listener check is not a reservation. Allocations are serialised by one lock that
    every worktree shares, so that two `up`s never search at once and take the same
    port; the plan says how long it is held. The lock lies outside any worktree; its
    path is the plan's to propose and step 5's to rule on (Open points).
  - Before it records the port, `up` confirms three things: the machine's actual
    mapping, that every listener on that port is this machine's, and a 200 with the
    token through it. How this machine's publisher is told from another process is the
    plan's to name, from smolvm's own account of the machine, as QS01 claims 5 and 16
    recorded it and step 2 confirms it, and never from `lsof` alone (Open points).
  - A collision is another process's listener on the candidate port. QS01 claim 5
    records that smolvm can bind beside one without an error, so a collision can show
    at three moments: as a machine that does not start, as a wait for the UI that times
    out, or at the confirmation. At each, `up` looks at the port before it reports: a
    failed start or a timeout with another process's listener on the candidate is
    handled as a collision, and is not reported as a failure of the machine. On a
    collision nothing of this worktree's is left served, and a machine `up` made in
    this run is deleted, so that none is left configured for a port it lost. A port set
    by hand is then a refusal, naming the process and `STUDIO_UI_PORT`. A searched port
    is released and the search runs again from the next candidate, and the machine is
    made again on the port it finds, with the data directory kept. The range bounds the
    retries.
  - There is one record of the port, beside the machine's state, tied to the machine's
    identity and checked against the actual mapping on every reuse. A later `up` of the
    same machine uses the recorded port again. If another process has taken it by then,
    `up` searches again.
  - For a machine that was there before this run, a port found by that second search is
    applied only if step 3's test allows relying on smolvm changing a stopped machine's
    port: a real run that shows the old mapping gone and the egress denials still in
    force (QS01's Open point 15 says why that is in doubt). Otherwise `up` refuses,
    naming the process that holds the port and `just studio down`.
  - `up` and `status` print one authoritative host URL, built from the confirmed port
    and the current token, and a separate line with the port alone, which Verification
    reads. QS05 is not a reader of that line: it reuses the convention of a separate
    port line only if it publishes a port of its own. Nothing parses the token URL for
    the port, and what `serve` prints itself, which names the guest's port, is never
    passed on as the host's URL.
- **The token** is read the way QS01 claim 5 found and shown in one place only: the URL
  line, which `up` and `status` print on standard output. `scripts/studio.sh` writes it
  to no file on the host, inside the worktree or outside it, and no step of this ticket
  does: output that would carry it is redacted before it reaches a file, under
  `ai_tmp/` as anywhere. Diagnostics go to standard error and never carry it, and guest
  log lines the script passes on have it replaced. In the guest it stays on the
  machine's own disk, never in the data directory or under another mount. Claim 5
  records when it changes, so the URL is built at each `up` and `status` and never
  stored.
- **Writes inside the worktree.** Under ruling 2 Studio can write only in the writable
  subdirectories. For each path Studio writes there, the ruling at step 5 picks:
  tracked, where the control is the maintainer reading the diff; ignored in
  `.gitignore`, with a comment; or turned off through Studio's settings. `just check`
  snapshots the worktree, so a path Studio writes that Git can see would fail it.
- **`just studio` reaches the network**: the first `up` pulls the image and installs
  from the package snapshot, and the guest has open egress. `AGENTS.md` lists it.
- **The page, `docs/how-to/run-quint-studio.md`**, registered in `docs/manifest.yml`
  as the owner of the topic and reachable from `docs/README.md`. It covers the two
  installs, `up`, signing in, opening the worktree, `stop`, `down`, the port rule and
  its override, where Studio's data and the machine's state live outside the worktree,
  deleting each and what that loses, and what the guest can read, write and reach,
  including what Studio fetches and sends at run time (ruling 9), citing decision 0022.
  `docs/how-to/develop-locally.md`, "Optional: Quint Studio", shrinks to a pointer at
  it.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 1 | The machine built from the Smolfile serves the UI on host loopback only, from listeners that are the machine's own, on a host port that is not 7000 | step 7, `curl` and `lsof` |
| 2 | Studio's sign-in and projects survive `stop` and `up`, and `down` then `up`, because Studio's data is in the data directory and not on the machine's disk | step 3, throwaway data read back after the machine is deleted and made again; step 8, the maintainer looking |
| 3 | From the guest, host loopback and the host's LAN address are unreachable, in each address family in which the guest has a route to the host, and a public HTTPS host is reachable | step 3, the control of QS01 claim 9 run on this layout's network, then the same probes under the denial; step 9, those probes on the built machine; in both, the host's listeners answering before and after, and each refusal in the machine's egress events |
| 7 | From the guest, a write to a tracked file, to `.git` and under each ignored tree the host runs fails, and the host's copy is unchanged; renaming a writable subdirectory, or an ancestor of one, fails; and each hidden path lists as empty, with a sentinel that is on the host before and after and absent in the guest | step 3 by hand and step 9 on the built machine, one attempt per path |
| 4 | A Studio session meant to change nothing changes nothing Git sees in the worktree: `git status --porcelain --untracked-files=all`, `git diff HEAD`, and the SHA-256 of every untracked file `git ls-files -o --exclude-standard` lists are the same before and after it. A session in which Studio makes one write in an approved writable subdirectory changes exactly that path where the ruling left it for Git to see, and nothing Git sees where the ruling ignored it | step 9, `cmp` on each pair, then the one expected difference, or the file shown directly beside three equal pairs |
| 5 | A file the guest writes in a writable subdirectory is owned on the host by the user who ran `just studio`, with a mode no wider than the guest asked for, and a write at the worktree root fails | step 9, `ls -ln` beside the mode the guest asked for and `id -u` |
| 6 | Two worktrees get two machines, two state directories, two data directories and two host ports, and data one guest writes to its data directory is absent from the other's | step 7, a second worktree booted |
| 8 | `up` refuses an existing machine whose recorded input digest differs from the one it computes, stopping it first if it runs, and before every `serve` checks that the extraction's stamp names the pinned AppImage's SHA-256: a missing or stale stamp, or a failed extraction, stops `serve` | step 6, the digest with the machine stopped and running, the stamp shown red after a restart, and the failed extraction under claim 15 |
| 9 | Studio serves, signs in, scans and runs its git commands with the worktree mounted read-only but for the writable subdirectories | step 3 by hand, step 8 on the built recipe |
| 10 | With `STUDIO_UI_PORT` unset, `up` takes the first free port of the range, records it and uses it again for that machine; a record that disagrees with the machine's actual mapping is refused by `up` and reported by `status`, each exiting 1; a port set by hand that is taken, or is 7000, is refused; and a port taken between the search and the boot is found before it is recorded, at the start, during the wait or at the confirmation: a searched port is searched again and the machine made again on the next, and a port set by hand is refused with no machine left | step 6, the first boot, the changed record and both collisions; step 7, the second worktree's port |
| 11 | A new machine from the pinned image, with only the snapshot's sources and fresh indexes, installs the guest's packages, and the resolved version and origin of each is recorded, none from a live mirror | step 3 by hand; step 7, on the machine its first `up` makes new |
| 12 | Through the wrapper, with the exact mounted common directory, new objects land only on the machine's disk and the host's objects are read; with another common directory nothing is redirected, one whose path begins as the mounted one's does included; a repository of the guest's own, and a caller that sets its own object directory, still work; and the whole sequence of git commands a scan runs succeeds | step 3 by hand; step 8, the scan's commands on the built recipe; step 9, the probes on the built machine |
| 13 | The layout's mounts, charged as QS01 claim 16 says, are within the budget, and `up` refuses a layout over it | step 3, the table against the machine's own listing; step 6, the refusal |
| 14 | `up` mounts, and `down` names for deletion, only a data directory whose identity record holds this worktree's physical path and the id the worktree keeps: a record that is missing, a symlink, or names another path or id is a refusal that names the way out, with nothing mounted and no deletion line printed; and the same checkout keeps its data across QS02's removal recipe | step 6, the record changed four ways; step 7, a value read back after `down`, the removal recipe and `up` |
| 15 | `up` never starts `serve` on a machine whose provisioning did not finish, at that run or a later one, even with a matching extraction stamp in place, and never reports as running a machine whose `serve` is dead | step 6, a failed package install and a failed extraction, each followed by a second `up`; `serve` killed in a running machine |

## Non-goals

- No MCP. Nothing here connects to Studio's MCP endpoint; QS05 does that.
- No guest image of this project's own. The packages come from the snapshot (ruling
  10); QS06 looks at an image.
- No CI job: no runner offers the hypervisor (decision 0022).
- No gate that runs Studio. `just check` neither needs nor touches the machine.
- No Linux host.
- No `CHANGELOG.md` entry.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review" (T37 is
still todo). One delta overrides them: the reviewer is Codex Sol, `-m gpt-6.2-sol`, in
place of the `-m gpt-6-astra` that AP01's command and §14 print, by the maintainer's
choice of 2026-11-25. AP01's fallback to a fresh-context `claude-opus-5-5` subagent
still applies, and the hand-back notes record the model that ran. `<id>` is `QS04`.
There are three checkpoints: at the end of step 2, and at steps 4 and 11.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tools/quint-studio/Smolfile` | new | the machine, with a comment naming decision 0022 |
| `tools/quint-studio/`, further files | new | the guest's git wrapper, and the provisioning step with its snapshot timestamp, under the names the plan gives and step 5's ruling accepts |
| `scripts/studio.sh` | new | `up`, `status`, `stop` and `down`, POSIX `sh` |
| `docs/how-to/run-quint-studio.md` | new | the how-to page |
| `Justfile` | shared, own lines | `studio *args`, with a comment |
| `.gitignore` | shared, own lines | only if the ruling ignores paths Studio writes in a writable subdirectory |
| `AGENTS.md` | shared, own lines | `just studio` in the list of recipes that reach the network |
| `docs/manifest.yml` | shared, own lines | the page's entry |
| `docs/README.md` | shared, own lines | a link to the page |
| `docs/decisions/0022-quint-studio-machine.md` | shared, own lines | Consequences: the data directory holds a credential, where it lives outside the worktree with its identity record, what `down` keeps and what removes it; the writable subdirectories; the hidden paths, and each path the guest can read that the ruling left unhidden; the port rule, with the lock every worktree shares; and the package snapshot |
| `docs/how-to/develop-locally.md` | shared, own lines | "Optional: Quint Studio" becomes a pointer at the new page |
| `docs/reference/commands.md` | shared, own lines | a `studio` row and the network list |
| `docs/reference/configuration.md` | shared, own lines | `STUDIO_UI_PORT` and the port rule, and a row for the Smolfile |
| `docs/explanation/security-model.md` | shared, own lines | what the guest can read, write and reach, the data directory, and the token |
| `docs/project/repository-map.md` | shared, own lines | the Smolfile, the wrapper, the provisioning step and `scripts/studio.sh` |
| `.scratch/quint-studio/QS04-just-studio.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS04's Status cell |

## Steps

Every quoted URL has its token replaced by `<redacted>`, and no file on disk holds a
token, under `ai_tmp/` included: output that carries one is redacted before it is
written, by a means that keeps the command's own exit status.

1. **Worktree, baseline and the facts.** Create the worktree on
   `ticket/qs04-just-studio` from a base that holds QS02 and QS03, and record the base
   and its full commit id. Show that the base also holds QS01's hand-back, with its
   "Rulings after the hand-back" section, and the README of these tickets with rulings
   1 to 10; if either is missing, **stop** for the maintainer, who names the base. Run
   `just initialize` (network, authorised), `just install-smolvm` and
   `just install-quint-studio` (network, authorised), and `just check` on the untouched
   worktree, and quote its last line. If `just install-quint-studio` fails on a hash
   mismatch, upstream has moved: **stop** for the maintainer's re-pin, by the procedure
   QS03 wrote. Read the hand-backs QS04 cites and write `ai_tmp/qs04/amendments.md`, as
   QS02's step 1 does. It holds each particular this ticket leaves to QS01's hand-back,
   with the claim or section it came from; the list of every `.gitignore` pattern, each
   with whether the host runs or loads what lies there and whether the guest may read
   it; the proposed writable list and hidden list; and the table of every mount with
   its cost against the budget.
2. **Read before writing** (writes outside the worktree, authorised by ruling 5: what
   QS02's hand-back lists its launcher as writing when `just smolvm` first runs in a
   worktree, the state directory with its marker and any parent made for it, the state
   link and the lock; nothing else). Confirm on the day, with `just smolvm --help` and
   the help of each subcommand used, every flag and Smolfile key the design needs;
   whether flags are accepted beside a Smolfile; whether a machine made from a Smolfile
   with no workload starts and takes guest commands; how smolvm itself reports a
   machine's port mapping and the process that publishes it; and whether a stopped
   machine's published port can be changed. Write `ai_tmp/qs04/plan.md`: the Smolfile's
   content, the provisioning step and the wrapper with their file names,
   `scripts/studio.sh`'s outline against "Design proposed" as amended, a proposal for
   each item of Open points, the means of each fault a later step injects, and the edit
   to each page in Files touched. It also names the provisional layout step 3 proves:
   every mount, the hidden paths with the place of the empty directory mounted over
   them, the writable subdirectories, the port range and the path of the lock. For the
   proof it names a data directory of the proof's own, at a path that does not exist
   yet and is not the one proposed for the recipe's, with the place of its identity
   record, and the control machine step 3 starts with. And it names the path proposed
   for step 7's second worktree, the commands that create and remove it, everything
   that worktree's run writes outside itself, and the one run of QS02's removal recipe
   that step 7 makes on this worktree's own state.

   Under a heading of its own the plan writes the request that step 3's stop makes:
   every path the proof writes outside the worktree, the launcher's own writes
   included, each with the command that removes it at the end of step 3 or the reason
   it stays; what the guest can read under the provisional hidden list, with open
   egress and before any ruling on that list, an ignored file at the worktree root and
   whatever a second mount shows included; and the control machine, which runs with no
   denial.

   **Adversarial review** (checkpoint 1: conformance and breakage, over that request
   alone, before step 3's stop makes it). Questions:
   - Does the request name everything the proof writes outside the worktree, and how
     each is removed or why it stays?
   - Could the proof's data directory be one that already holds something, or the one
     the recipe will use?
   - Under the provisional layout, can the guest read a credential or private data
     before the hidden paths are proved?
   - Can the control machine mount anything, reach more than its probes need, or
     outlive them?
3. **The layout, proved by hand** (network, authorised: the image pull, the package
   snapshot and the guest's egress, the control machine's included, which has no
   denial; writes outside the worktree, authorised by ruling 5 and by the leave asked
   below: what step 2 names for the launcher, the proof's data directory with its
   identity record, and the empty directory mounted over the hidden paths if the plan
   places it outside; nothing else). No tracked file changes. Before the first command,
   **stop for the maintainer**: quote the plan's request as checkpoint 1 left it and
   ask for leave. The maintainer may add a path to the hidden list, or move a file
   aside, before anything boots, and the plan is amended to match. The maintainer is
   needed again for the sign-in below.

   Show first that the proof's data directory does not exist (`test ! -e`), and run,
   read-only, the Rosetta preflight "Facts for QS02" describes; a host that fails it is
   a **stop**. Save each command, its output and its exit status under
   `ai_tmp/qs04/proof/`, with any token redacted before the output reaches a file. The
   parts run in this order, each on the machine the part before it left, and `serve` is
   not started until every part above it has passed:
   - **The control** (claim 3), before the proof's machine exists: "Facts for QS04"
     says how many machines a state root holds while one mounts the worktree. Start the
     listeners of QS01 step 10 on host loopback and on the host's LAN address, in each
     address family the host has there, and show each answering the host itself.
     Through `just smolvm`, make a control machine from the pinned image with
     networking on, neither denial and no mount, and from it find each guest-side
     address, in each family, that reaches each listener, as QS01 claim 9's control
     did. The plan names the client the guest probes with, which the guest the recipe
     builds must also carry; if the image lacks it, the control machine installs it
     from the snapshot alone. Record the addresses with the guest's gateway and
     resolver. Delete the control machine and show it gone from smolvm's own listing.
     If the guest reaches the host in a family the draft's denial does not name, amend
     the draft before going on and say so in the plan; if the documented settings
     cannot deny it, **stop** (Open points).
   - **The machine**: make by hand each host directory the layout needs that is
     absent, the proof's data directory and its identity record among them. Then,
     through `just smolvm`, make a new machine from the pinned image and the draft
     Smolfile, in the form `up` will use, with the plan's provisional layout, and start
     it. Show that nothing serves: no `serve` process in the guest.
   - **The mounts** (claim 13): the machine's own listing of its mounts, read the way
     QS01 claim 16 says each kind is listed, agrees with the plan's table, within the
     budget.
   - **The hidden paths** (claim 7): for each, a harmless sentinel made on the host
     under it is present on the host, absent in the guest, where the path lists as
     empty, and still present on the host afterwards. The plan's list of what the guest
     can read is checked from inside the guest, the paths other mounts show included.
   - **The read-only mount** (claim 7): from the guest, a write in each writable
     subdirectory succeeds and the host sees it; a write to a tracked file, to `.git`
     (the pointer file opened for writing where it is a file, a file created in it
     where it is a directory), a file created in the common git directory, and a write
     under each ignored tree the host runs all fail, with the SHA-256 of the host's
     copy unchanged; renaming each writable subdirectory, and each ancestor of one,
     fails.
   - **The data directory** (claim 2): a file holding a random value, written from the
     guest at the path of Studio's data, is on the host in the proof's data directory.
     Delete the machine, make it again and start it, as `down` and `up` will do, and
     show the guest reading the same value back. The parts below run on the machine
     made again, which is running with nothing serving.
   - **The packages** (claim 11): provision that machine. Its package sources hold the
     snapshot alone, the indexes are fresh, the install exits 0, and a file records the
     resolved version and the origin of every package installed, none from a live
     mirror. A failed install is a **stop**; there is no fallback to a live mirror.
     Then the wrapper is installed, the AppImage extracted with its stamp, and the
     completion record written last.
   - **The wrapper** (claim 12): with the exact mounted common directory, an object
     the guest adds lands on the machine's disk, nothing new appears in the host's
     common directory, and an object of the host's is read; in a repository made in
     the guest nothing is redirected, nor in a second one made in the guest at a path
     that begins as the mounted common directory's does, which a comparison by
     beginning, such as "Facts for QS04" reports of the wrapper QS01 tested, would take
     for the mounted one; a call that sets its own object directory keeps it.
   - **The reach** (claim 3): with each listener shown answering the host, each
     guest-side address the control found fails from this guest, in each family, and a
     public HTTPS host answers. The listeners answer the host again afterwards, so that
     a failure is the policy's and not a dead listener's, and the machine's egress
     events, read the way claim 9 read them, list each refusal. The client is the
     control's.
   - **`serve`**: the completion record and the extraction's stamp are checked first;
     then `serve` is started by the one guest command, and through the port the machine
     publishes, a free one of the proposed range, the UI answers 200 with the token and
     the refusal QS01 claim 5 recorded without it. Every listener on that port is this
     machine's, by the means the plan names.
   - **The port change**, only if the plan relies on it: stop the machine, change its
     published port by smolvm's own command, start it, pass the checks before `serve`
     and start `serve` again. Show the old mapping gone and the new one answering, and
     run the reach part again in full, with the denials still in force. Without that
     evidence the design takes the refusal path.
   - **Studio itself** (claim 9). **Stop for the maintainer**, who opens the URL, signs
     in and runs one scan to its end, with the proof's data directory mounted. Then
     show the scan's git commands, each with its exit status, all 0, from a record the
     proof's copy of the wrapper may keep, and list from Studio's logs every path it
     tried to write and could not. A path it needs inside the worktree joins the
     proposed writable list for step 5's ruling. Record nothing the maintainer typed.

   Any failure, or a writable path that cannot be settled, is a **stop** for the
   maintainer; the builder does not choose. The fallbacks to put to the maintainer are
   named in Open points: for the read-only worktree, the other two ways QS01's "Go or
   no-go" lists; for the data directory, another storage design, or a `down` that
   deletes the data and says so. Write the results into `ai_tmp/qs04/plan.md`, so that
   checkpoint 2 reviews them. Then delete the proof's machine, and remove the proof's
   data directory with its identity record, the sign-in in it included, and the empty
   directory if it lies outside, each by the command the request quoted, and show each
   gone (`test ! -e`). What the launcher wrote stays, since the recipe uses it. "Facts
   for QS04" says how many machines a state root holds while one mounts the worktree,
   and step 8's sign-in is on the built recipe.
4. **Adversarial review** (checkpoint 2: conformance and breakage, over
   `ai_tmp/qs04/plan.md`, with step 3's results, and `ai_tmp/qs04/amendments.md`, before
   any tracked file changes). Questions:
   - Can `up` boot without both stamped installs, or serve a tree extracted from
     anything but the pinned AppImage?
   - Can `up` start `serve` on a machine whose provisioning did not finish, or report
     as running a machine whose `serve` is dead?
   - Can the UI port be bound on an address other than host loopback, or on host port
     7000?
   - Can two `up`s, in two worktrees or beside another project, end on the same host
     port, or can `up` record a port whose listener is not its own machine's?
   - Can a collision, at the start, during the wait or at the confirmation, leave a
     machine configured for a port it lost, or read as a failure of the machine?
   - Can the guest reach host loopback or a private range, in any address family, or
     write the worktree outside the writable subdirectories, or the common git
     directory?
   - Can a port check that fails, or a wait that times out, read as success?
   - Can `down` lose the sign-in, or can any action delete the data directory?
   - Can a checkout mount, or `down` name for deletion, a data directory that is not
     its own, or can the guest rewrite that directory's identity record?
   - Can a path Studio writes make `just check` fail?
   - Does the hidden list miss a path the guest must not read, or does another mount
     show a hidden tree?
   - Can `up` boot a layout over the mount budget?
   - Can the wrapper redirect a repository other than the mounted one, or let an object
     reach the host's common directory?
   - Can a package come from anywhere but the snapshot?
   - Can the token reach a file on disk, through the recipe, a diagnostic, or the
     proof's or Verification's output?
   - Can an existing machine serve a Studio, a Smolfile or a mount other than the ones
     `up` checked, or can two worktrees share a data directory?
5. **The ruling.** **Stop** for the maintainer's ruling on "Design proposed" as amended,
   with step 3's results and checkpoint 2's findings in hand, and record it. It covers
   the writable list; the hidden list, with each path the guest can read that is left
   unhidden; tracked, ignored or turned off for each path Studio writes; the port
   range, the override and the reuse; the paths of the data directory, of its identity
   record and of the lock; and each question under Open points. The same stop asks for
   the path of step 7's second worktree, which lies outside this one, for leave to
   create and remove it there, and for leave for what its run writes outside itself:
   what QS02's hand-back lists the launcher as writing for it, its data directory with
   the identity record, and its empty directory if the ruling places that outside.
   Each is removed before its checkout is, but for what QS02's hand-back says its
   removal recipe leaves behind, which the request names with the command that removes
   it or the reason it stays. The second worktree takes the same lock for port
   allocation, which is one for all worktrees and stays. The stop also asks for leave
   to run QS02's removal recipe once on this worktree's own state, at step 7, where
   the `up` after it makes that state again. Checkpoint 2's brief names these requests,
   so no other review is needed before them. If the ruling changes anything step 3
   proved, the writable list, the hidden list or the place of the empty directory, a
   mount or the layout, the data directory's path or its record's, the network
   settings, the port range or the form of the Smolfile, run again the parts of step 3
   the change touches, and all of them if a mount changes, on the changed layout before
   step 6, under the leave this stop gives, and write the results into the plan.
6. **The machine and the recipe** (network, authorised, from the first bullet that
   boots: the image pull, the package snapshot and the guest's egress; writes outside
   the worktree, authorised at step 5: what QS02's hand-back lists the launcher as
   writing for this worktree, the data directory with its identity record, the lock
   for port allocation, and the empty directory if the ruling placed it outside;
   nothing else). Add the Smolfile, the wrapper, the provisioning step,
   `scripts/studio.sh` and `just studio`. No machine exists when this step starts,
   since step 3 deleted the proof's: `just studio status` prints the line it gives for
   no machine, quoted once as the control. Each bullet says the state it starts from
   and the state it leaves, and "boots nothing" means that `just studio status` prints
   that line afterwards. Each fault is injected by the means the plan names and
   checkpoint 2 reviewed. Show, quoting each run and its exit status:
   - no machine, before and after: with `.tools/smolvm/.pachinko-stamp` moved aside,
     `just studio up` exits 1 naming `just install-smolvm`; with
     `.tools/quint-studio/.pachinko-stamp` moved aside, it exits 1 naming
     `just install-quint-studio`; each boots nothing, and each stamp is restored
     afterwards (`cmp` exits 0);
   - no machine, before and after: with the Rosetta preflight made to fail,
     `just studio up` exits 1 naming the recovery and boots nothing;
   - no machine, before and after: with a listener on a port of the range, started from
     the pixi environment with `python -m http.server --bind 127.0.0.1 <port>`, and
     `STUDIO_UI_PORT` set to that port, `just studio up` exits 1, naming the process
     and `STUDIO_UI_PORT`, and boots nothing;
   - no machine, before and after: `STUDIO_UI_PORT=80`, `STUDIO_UI_PORT=70000`,
     `STUDIO_UI_PORT=abc` and `STUDIO_UI_PORT=7000` each exit 1, naming
     `STUDIO_UI_PORT`, and boot nothing;
   - no machine, before and after: with an `lsof` that fails, `just studio up` exits 1
     naming `lsof` and boots nothing, with a message other than the one for a taken
     port;
   - no machine, before and after: with a hidden path moved aside and replaced by a
     symlink to a new, empty directory, which lies outside the path it replaces, and
     again with a writable subdirectory, and again with an ancestor of one of them,
     `just studio up` exits 1 naming the recovery, boots nothing, and the target stays
     empty; each restored afterwards;
   - claim 13, no machine, before and after: with the layout lengthened in the working
     tree until it is one mount over the budget, `just studio up` exits 1 naming the
     budget and boots nothing; restored afterwards (`cmp` exits 0);
   - claim 10, from no machine to a machine serving: with a listener held on the first
     free port of the range and `STUDIO_UI_PORT` unset, `just studio up` takes the next
     free one and says so, and `just studio status` prints the same port line. After
     `just studio stop`, with that listener gone, `just studio up` uses the recorded
     port again. Then, with the port's record changed to another port of the range and
     the original saved, `just studio status` exits 1 naming the mismatch, and
     `just studio up` exits 1 naming `just studio down` and leaves the machine stopped;
     with the record restored (`cmp` exits 0), `just studio up` serves again;
   - claim 10, the collisions, each from no machine, so `just studio down` first. With
     `STUDIO_UI_PORT` unset, a process takes the candidate port between the search and
     the boot, bound on host loopback so that the machine's start or the wait for the
     UI meets it: `up` reports a collision, not a failed start or a timeout, deletes
     the machine it made, searches again, makes the machine again on the next free
     port and serves, exiting 0; `just studio status` prints the new port line, and
     smolvm's own listing, through `just smolvm`, shows one machine, mapped to that
     port. Then `just studio down`, and with `STUDIO_UI_PORT` set to a free port, a
     process takes that port between the check and the boot, bound the way QS01 claim
     5 saw another process hold a port beside smolvm's, so that the machine starts and
     the collision shows at the confirmation: `up` exits 1 naming the process and
     `STUDIO_UI_PORT`, records no port and boots nothing, the machine it made being
     gone;
   - claim 14, no machine, before and after, on the data directory the runs above made,
     which holds no sign-in yet: its identity record changed to another id, then to
     another path, then removed, then replaced by a symlink to a copy of it, with the
     original saved first and restored after each (`cmp` exits 0). Each time
     `just studio up` exits 1 naming the directory, what its record says and the way
     out, and boots nothing, and `just studio down` prints no deletion line for the
     directory;
   - claim 15, from no machine: with the package install made to fail, `just studio up`
     exits 1 with nothing served and the machine stopped; with the fault removed, a
     second `just studio up` exits 1 naming `just studio down`, `serve` never starts
     and the machine is stopped afterwards. After `just studio down`, the same two runs
     with the extraction made to fail. Then, with that machine still there and stopped,
     start it through `just smolvm`, write by a guest command a stamp holding the pinned
     AppImage's SHA-256 where the extraction writes its own, and stop it through
     `just smolvm`: `just studio up` still exits 1, since the completion record is
     missing, and `serve` never starts. After `just studio down`, `just studio up`
     serves. Then, with the machine serving, kill `serve` by a guest command through
     `just smolvm`: `just studio up` does not report the machine as running; it stops
     it, starts it and serves again, and the URL it prints answers 200. That leaves a
     machine serving;
   - claim 8, the digest, from a machine serving: `just studio stop`. With the machine
     stopped, the recorded digest changed by one character, then the record removed,
     then the Smolfile changed by one comment byte, each restored before the next
     (`cmp` exits 0): each time `just studio up` exits 1 naming `just studio down`, and
     the machine stays stopped. Then `just studio up`, which serves, and the same three
     changes with the machine running, with `just studio up` after each restoration so
     that the next starts from a machine serving: each time `up` stops the machine
     first and exits 1 naming `just studio down`, `just studio status` reports it
     stopped, and `lsof -nP -iTCP:<port> -sTCP:LISTEN`, on the recorded port, finds no
     listener. That leaves a machine serving;
   - claim 8, the stamp, from a machine serving: `just studio stop`. Start the stopped
     machine through `just smolvm`, which starts no `serve` (show no `serve` process in
     the guest), remove the extraction's stamp by a guest command, and stop the machine
     through `just smolvm`. Then `just studio up` exits 1 naming the recovery, `serve`
     never starts, the machine is stopped afterwards and the same `lsof` finds no
     listener. The same again, from that stopped machine, with a stamp holding another
     digest written in its place. After `just studio down` and `just studio up` it
     serves again, which is the state this step leaves;
   - from any state, which they leave as it was: `just studio` alone and
     `just studio restart` each print the four actions and exit non-zero.
7. **Up** (network, authorised: the image pull, the package snapshot and the guest's
   egress; writes outside the worktree, authorised at step 5: what step 6 names for
   this worktree, and the second checkout at the path the ruling named, with what the
   ruling allowed its run to write outside itself; nothing else. By the same leave,
   QS02's removal recipe runs once on this worktree's state). Step 6 leaves a machine
   serving. From its guest, write a file holding a random value into Studio's data.
   Then run `just studio down` and QS02's removal recipe for this worktree's state, and
   show `just studio status` printing the no-machine line: the `up` that follows makes
   the state and its machine new. Then `just studio up` prints the host URL and the
   port line, and the guest reads the same value back, so the data directory was taken
   as this checkout's own after the removal recipe (claim 14). From the host, `curl` of
   the UI with the token answers 200 and without it gives exactly the status, and any
   redirect target, QS01 claim 5 recorded;
   `lsof -nP -iTCP:<port> -sTCP:LISTEN`, on the port `up` printed, shows listeners on
   host loopback only, in the address families claim 5 recorded, each of them the
   machine's own by the means the ruling accepted, and the port is not 7000 (claim 1).
   Claim 11: on this machine, which that `up` made new from the pinned image, quote
   the record of each package's resolved version and origin, and show every origin is
   the snapshot. Claim 6: show the file that holds the random value on the host, in
   this worktree's data directory. The second worktree
   needs the implementation under test, so first commit the work so far on the ticket
   branch (step 12's commit builds on it). Create the second worktree at the path step
   5's ruling named, with `git worktree add --detach <path> HEAD`, show
   `git -C <path> rev-parse HEAD` equal to this worktree's, and in it run
   `just initialize`, `just install-smolvm` and `just install-quint-studio` (network,
   authorised). `just studio status` there reports no machine; then, with this
   worktree's machine still running and `STUDIO_UI_PORT` unset, `just studio up` there
   (network, authorised: its own image pull, snapshot and egress; writes outside both
   worktrees, authorised at step 5: what the ruling allowed the second worktree's run)
   boots its own machine on its own port. Show the two machine names, the two state
   directories, the two data directories and the two port lines differing, and the
   file absent from the second guest's data. Then, in the second worktree:
   `just studio down`; the deletion of its data directory, with the identity record,
   that `down` printed, which loses no sign-in; the removal of its empty directory, if
   the ruling placed that outside; and QS02's removal recipe for its state. Show its
   state link gone without following it (`test ! -L` and `test ! -e`), and its state
   directory, its data directory and that empty directory gone (`test ! -e`). Show
   what QS02's hand-back says the removal recipe leaves behind, and remove it or leave
   it, as step 5's ruling said. Only then `git worktree remove <path>` from this one,
   and `test ! -e <path>`; quote each. This worktree's machine is still serving.
8. **Sign-in and persistence** (network, authorised: `up` after `down` makes the
   machine again, so the image, the package snapshot and the guest's egress; writes
   outside the worktree, authorised at step 5: what step 6 names, with the maintainer's
   sign-in and projects now in the data directory). **Stop for the maintainer**, who
   opens the URL, signs in, configures the AI provider, opens this worktree as a
   project and runs one scan to its end (claim 9, on the built recipe). Then show the
   scan's git commands, each with its exit status, all 0, from the record step 5's
   ruling accepted (claim 12). Then `just studio stop`, `just studio up`: the
   maintainer confirms Studio is still signed in with the project listed. Then
   `just studio down`, `just studio up`: the same, and the maintainer says whether the
   project and its scan read as before, or what was lost with the machine's disk
   (claim 2). Record nothing the maintainer typed.
9. **Reach and writes**, on the machine step 8 left serving. Claim 3: read the guest's
   gateway and resolver and show them to be the ones step 3's control recorded. If they
   differ, or the network settings changed after that control, run the control again
   first, with this machine stopped ("Facts for QS04" says why), and start it again
   with `just studio up`. Start QS01 step 10's listeners as step 3 did and show each
   answering the host. From the guest, each guest-side address the control found fails,
   in each address family, and the public request answers; the listeners answer the
   host again afterwards; and the machine's egress events list each refusal. Claim 4:
   later edits may not yet be committed, so the check compares rather than asks for an
   empty status. Before a Studio session in which the maintainer opens a file and runs
   one action meant to change nothing, save to `ai_tmp/qs04/`
   `git status --porcelain --untracked-files=all`, `git diff HEAD`, and
   `shasum -a 256` of every untracked file `git ls-files -o --exclude-standard` lists,
   which covers the new Smolfile, script and page that `git diff HEAD` does not; after
   it, save all three again, and show `cmp` exiting 0 on each pair. Then have Studio
   make one write in an approved writable subdirectory, and save all three once more.
   Where the ruling left that path for Git to see, show that the three differ from the
   last by exactly that path. Where the ruling ignored it, Git's views cannot show it:
   show the three equal to the last (`cmp` exits 0) and the file itself, by `ls -l` and
   its SHA-256 before and after. Where the ruling turned Studio's writes off, record
   that there is none to make; the probe below is then the only write. Restore or
   commit the change before any `just check`. The empty status after the commit is
   Verification's. Have the guest create `qs04-owner-probe` in a writable subdirectory
   with a mode the plan names, show `ls -ln` of it on the host beside that mode and
   `id -u` of the user who ran `just studio`, and remove it; then show the guest
   failing to create it at the worktree root, and no such file on the host (claim 5).
   Repeat step 3's probes of the read-only mount and of each hidden path on this
   machine, one attempt per path, each quoted (claim 7), and step 3's wrapper probes,
   the repository whose path begins as the mounted one's does among them (claim 12).
   Run `just check` on the host while Studio runs, and quote its last line.
10. **Words.** Write the page and edit each page in Files touched, following
    `.agents/skills/review-docs/SKILL.md`. Run `just check-docs` and
    `just check-agents`.
11. **Adversarial review** (checkpoint 3: conformance and breakage, over the diff with
    the pages included, before the final commit). The README of these tickets, "Rules
    every ticket holds", binds this ticket here as at every step: its stop on a red
    baseline, its heading for every quote, its rerun of the evidence that a fix made at
    this checkpoint touches, and its rerun before any request to push. Questions:
    - Could Verification pass while the UI was published beyond host loopback, or on
      host port 7000, or while the tree served was not the pinned AppImage's?
    - Does any committed file, quoted output or page, or any file under `ai_tmp/qs04/`,
      hold a token?
    - Does `just check` stay free of the network and of the machine?
    - Does the page say, plainly, what the guest can read, write and reach, where
      Studio's data and the machine's state live outside the worktree, and what
      deleting each loses?
    - Does anything written outside the worktree lack a line in decision 0022 or a
      command that removes it?
12. **Commit and hand back** (network, authorised: Verification's `up` makes the
    machine again; writes outside the worktree, authorised at step 5: what step 6
    names). Commit the work. Run `just studio down`, which keeps the data directory, as
    step 8 showed; Verification starts with the machine down. Run Verification and
    quote it, headed by the full commit id; it ends with its own `just studio down`.
    Set `status: done` here and in `.scratch/quint-studio/README.md`, write the
    hand-back notes, and commit the record.
    The notes name what this ticket leaves outside the worktree, and the command that
    removes each: what QS02's launcher wrote, the data directory with its identity
    record and the maintainer's sign-in, the lock for port allocation, and the empty
    directory if it lies outside. On the record commit, run
    `git diff --name-only HEAD~1 HEAD`, which must list only this ticket and the README,
    then `just check` and `git status --short`, and report their output to the
    maintainer with the hand-back, headed by that commit, since a commit cannot quote
    its own check. If any fails, fix the cause in a new commit and run all three again.
    **Stop** before pushing or opening a pull request: both need authorisation
    (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `just studio up` refuses without either stamped install, on a host that fails the
  Rosetta preflight, on a `STUDIO_UI_PORT` that is taken, malformed or 7000, on a
  failed `lsof`, on a symlink at a hidden path, a writable subdirectory or an ancestor
  of one, on a layout over the mount budget, on a data directory whose identity record
  is missing, a symlink or another checkout's, over a machine whose input digest
  differs or whose port record disagrees with its actual mapping, stopping that machine
  if it runs, and over a machine whose completion record or extraction stamp is missing
  or stale, each with exit 1, its diagnostic and nothing of this worktree's served
  afterwards, and boots otherwise, checking the completion record and the extraction's
  stamp against the pinned AppImage's SHA-256 before `serve` and printing the host URL
  and the port line, each quoted with the token redacted. `status` exits 1 on the same
  disagreement.
- The refusal over a data directory that is not this checkout's names the way out and
  mounts nothing, `down` prints no deletion line for such a directory, and the same
  checkout's data is read back after `down`, QS02's removal recipe and `up`, each
  quoted.
- After a failed package install, and after a failed extraction, a second `up` refuses
  and `serve` never starts, even with a matching stamp put in place by hand; and a
  running machine whose `serve` is dead is not reported as running, each quoted.
- With `STUDIO_UI_PORT` unset, `up` takes the first free port of the ruled range, never
  7000, records it and uses it again for that machine. A port taken between the search
  and the boot is found before it is recorded: a searched port is searched again and
  the machine made again on the next, and a port set by hand is refused with no machine
  left, each quoted.
- The UI answers on host loopback only, from listeners that are the machine's own, with
  200 only with the token, and without it exactly the refusal QS01 claim 5 recorded: 401
  or 403, or a redirect to `127.0.0.1`.
- No file on disk holds a token, under `ai_tmp/` included, and Verification's own search
  of its output for the token finds none.
- Two worktrees get two machines, two state directories, two data directories and two
  host ports, and one guest's data is absent from the other's, quoted; the second
  worktree's state and data outside it are shown removed before its checkout is, and
  what QS02's removal recipe leaves behind is named.
- Studio serves, signs in, scans and runs its git commands with the worktree mounted
  read-only but for the writable subdirectories, by step 3's proof and again on the
  built recipe, as the maintainer confirms.
- Studio's sign-in and projects survive `stop` and `up`, and `down` and `up`, as the
  maintainer confirms.
- The guest cannot reach host loopback or the host's LAN address, in any address family
  in which the control reached them, with the host's listeners answering before and
  after and each refusal in the machine's egress events, and can reach a public HTTPS
  host, each quoted.
- From the guest, a write to a tracked file, to `.git`, to the common git directory and
  under each ignored tree the host runs fails with the host's copy unchanged, renaming
  a writable subdirectory or an ancestor of one fails, and each hidden path lists as
  empty with its sentinel untouched on the host, each quoted. Decision 0022 and the
  page name each path the guest can read that the ruling left unhidden.
- The guest's packages are installed on a new machine from the snapshot alone, with the
  resolved version and origin of each quoted from the record, and none from a live
  mirror.
- Through the wrapper, new objects of the mounted repository land only on the machine's
  disk, nothing is redirected for another repository, one whose path begins as the
  mounted one's does included, a caller's own object directory is kept, and a scan's
  git commands all exit 0 on the built recipe, each quoted.
- A Studio session meant to change nothing changes nothing Git sees in the worktree. One
  write by Studio in a writable subdirectory changes exactly that where Git sees the
  path, and where the ruling ignored the path it leaves Git's three views equal and is
  shown directly, by step 9's comparisons.
- `docs/how-to/run-quint-studio.md` is registered and reachable, and `just check-docs`
  passes.
- `AGENTS.md` lists `just studio` among the recipes that reach the network, and
  `just check-agents` passes.
- Each claim has its outcome recorded.
- The three checkpoints are recorded as §14's Record asks, with AP01's deltas.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record
  commit, as step 12 reports.

## Verification

Run from a file as one `sh` script after the commit and after step 12's
`just studio down`, so with the machine down; `set -eu` stops it at the first command
that fails. It is never run with `-x`, which would print the token. It reaches the
network (the image, the package snapshot and the guest's egress, since its `up` makes
the machine again), under the ticket's authorisation, and writes outside the worktree
only what step 6 names for `just studio up`. No pipeline carries a result: each command
writes to a file, its exit status is checked, and the file is matched exactly. There is
one exception to "writes to a file", made once for each start of the machine. What
`just studio status` prints while the machine runs holds the token URL, and no file may
hold a token. So `read_status` takes that output into a shell variable by one command
substitution, whose exit status `set -e` checks, and splits it in memory.

```sh
set -eu
unset STUDIO_UI_PORT
git rev-parse HEAD
no_machine='<the line just studio status prints for no machine, quoted at step 6>'
port_label='<the fixed text before the number on the port line, quoted at step 6>'
range_first='<the first port of the range step 5 ruled>'
range_last='<the last port of that range>'
loopback='<the loopback addresses of QS01 claim 5, as lsof prints them, spaced apart>'
refused='<the HTTP status QS01 claim 5 recorded for a request without the token>'
location='<the redirect target QS01 claim 5 recorded for it, or empty if none>'
stamp=.tools/quint-studio/.pachinko-stamp
out=ai_tmp/qs04/verify
mkdir -p "$out"
restore() {
    if [ -e "$out/studio-stamp" ]; then mv "$out/studio-stamp" "$stamp"; fi
}
trap restore EXIT
nl='
'

# Reads what status prints while the machine runs, in memory: the port line into
# port_line, the token URL into url, and a copy for "$1.txt" in which every line
# that holds a token is cut off before it.
read_status() {
    shown=$(just studio status 2>| "$out/$1.err")
    port_line='' url='' port_lines=0 idle=0
    : >| "$out/$1.txt"
    old_ifs=$IFS
    IFS=$nl
    set -f
    for l in $shown; do
        case "$l" in
        "$no_machine") idle=$((idle + 1)) ;;
        "$port_label"*)
            port_line=$l
            port_lines=$((port_lines + 1))
            ;;
        *token=*)
            url=${l#*http://}
            url="http://${url%%[[:space:]]*}"
            l="${l%%token=*}token=<redacted>"
            ;;
        esac
        printf '%s\n' "$l" >> "$out/$1.txt"
    done
    set +f
    IFS=$old_ifs
    shown=''
    test "$idle" -eq 0
    test "$port_lines" -eq 1
    test -n "$url"
}

# Stops if a file under $out holds the token just read. The first grep is the
# positive control: the same search finds the port label, which is there.
no_token_on_disk() {
    token=${url#*token=}
    test -n "$token"
    grep -rqF -e "$port_label" "$out"
    if grep -rqF -e "$token" "$out"; then exit 1; else test "$?" -eq 1; fi
    token=''
}

# 200 with the token, and the recorded refusal without it, through the port in $port.
probe() {
    case "$url" in
    "http://127.0.0.1:$port/?token="?*) ;;
    *) exit 1 ;;
    esac
    bare="http://127.0.0.1:$port/"
    code=$(curl -sS -o /dev/null -w '%{http_code}' "$url")
    test "$code" = 200
    answer=$(curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' "$bare")
    test "$answer" = "$refused $location"
}

# Control: with the machine down, status prints its no-machine line.
just studio status >| "$out/status-down.txt" 2>&1
grep -Fx "$no_machine" "$out/status-down.txt"

# Red: without the Studio stamp, up refuses and boots nothing.
mv "$stamp" "$out/studio-stamp"
rc=0; just studio up >| "$out/refuse-stamp.log" 2>&1 || rc=$?
mv "$out/studio-stamp" "$stamp"
test "$rc" -eq 1
grep -F 'just install-quint-studio' "$out/refuse-stamp.log"
just studio status >| "$out/status-stamp.txt" 2>&1
grep -Fx "$no_machine" "$out/status-stamp.txt"

# Red: a malformed port is refused and boots nothing.
rc=0; STUDIO_UI_PORT=abc just studio up >| "$out/refuse-port.log" 2>&1 || rc=$?
test "$rc" -eq 1
grep -F 'STUDIO_UI_PORT' "$out/refuse-port.log"
just studio status >| "$out/status-port.txt" 2>&1
grep -Fx "$no_machine" "$out/status-port.txt"

# Red: host port 7000 is refused when set by hand, and boots nothing.
rc=0; STUDIO_UI_PORT=7000 just studio up >| "$out/refuse-7000.log" 2>&1 || rc=$?
test "$rc" -eq 1
grep -F 'STUDIO_UI_PORT' "$out/refuse-7000.log"
just studio status >| "$out/status-7000.txt" 2>&1
grep -Fx "$no_machine" "$out/status-7000.txt"

# Green: up boots. Its standard output carries the token URL and is not kept, and
# status then prints a port of the range, never 7000, on a line of its own.
just studio up > /dev/null 2>| "$out/up.err"
read_status status-up
no_token_on_disk
printf '%s\n' "$port_line" >| "$out/port-line.txt"
port=${port_line#"$port_label"}
case "$port" in
'' | *[!0-9]*) exit 1 ;;
esac
test "$port" -ne 7000
test "$port" -ge "$range_first"
test "$port" -le "$range_last"

# Green: the listeners on that port are on host loopback only, and one process's.
lsof -nP -iTCP:"$port" -sTCP:LISTEN >| "$out/lsof.txt"
: >| "$out/allowed.txt"
set -f
for addr in $loopback; do
    printf '%s:%s (LISTEN)\n' "$addr" "$port" >> "$out/allowed.txt"
    grep -F "$addr:$port (LISTEN)" "$out/lsof.txt"
done
set +f
test -s "$out/allowed.txt"
rc=0; grep -v -F -f "$out/allowed.txt" "$out/lsof.txt" >| "$out/lsof-other.txt" || rc=$?
test "$rc" -le 1
if grep -F '(LISTEN)' "$out/lsof-other.txt"; then exit 1; else test "$?" -eq 1; fi
awk '/\(LISTEN\)/ { n++; if (!($2 in seen)) { seen[$2] = 1; pids++ } }
    END { exit !(n > 0 && pids == 1) }' "$out/lsof.txt"

# Green: 200 only with the token, through the port status printed.
case "$refused" in
401 | 403) test -z "$location" ;;
301 | 302 | 303 | 307 | 308)
    case "$location" in
    "http://127.0.0.1:$port/"?*) ;;
    *) exit 1 ;;
    esac
    ;;
*) exit 1 ;;
esac
probe

# Green: after a stop, up uses the same port again and Studio answers through it.
# QS01 claim 5 records when the token changes, so it is read again.
just studio stop
just studio up > /dev/null 2>| "$out/up-again.err"
read_status status-again
no_token_on_disk
test "$port_line" = "$port_label$port"
probe
just studio down
just studio status >| "$out/status-after.txt" 2>&1
grep -Fx "$no_machine" "$out/status-after.txt"

just check
status=$(git status --short)
test -z "$status"
```

The seven placeholders are filled in before the script runs: `no_machine` and
`port_label` from step 6's quotes, the range from step 5's ruling, and `loopback`,
`refused` and `location` from QS01's hand-back. The script unsets `STUDIO_UI_PORT`, so
the green run is the search, and it takes the port from the port line alone: the URL
must carry that port and never supplies it. The port line must be the only line
`status` prints that begins with `port_label`, or the script stops.

No file holds the token. `up`'s standard output goes to `/dev/null`: step 7 quotes
what `up` prints, redacted, and here `status` is the reader. `read_status` holds what
`status` prints in memory, writes each line to its file with everything from `token=`
on replaced, and keeps the URL in `url`, which is never echoed. It uses no
here-document, which a shell may back with a temporary file. Standard error is kept in
files, since by the design it never carries the token, and so are the refusal logs,
which are written before anything boots. `no_token_on_disk` checks all of that after
each read: it searches every file under `ai_tmp/qs04/verify/` for the token just read
and stops on a match, after the same search has found the port label there. If it
stops the script, a file there holds a token: remove `ai_tmp/qs04/verify/` and run
`just studio down` before anything else.

Each `grep` inside the loop is a positive control for the silent probe after it: every
loopback address claim 5 recorded must have its listener, the probe reads the same
output, and `grep -v` failing with status 2 stops the script. The loop runs under
`set -f`, so an address is never taken as a file pattern, and `test -s` stops a
`loopback` left empty. The `awk` line stops unless the listeners on the port belong to
one process. That the process is this machine's is `status`'s own confirmation, which
exits non-zero when it fails and which step 6 shows red; the script holds no account of
the machine's publisher to match `lsof` against (Open points). The request with the
token is the positive control for the one without it, which must give exactly the
recorded answer, and `probe` makes both again after the restart, with the token read
again. The `case` on `refused` checks that recorded answer independently: only 401,
403, or a redirect to a path on the same loopback address other than the bare UI counts
as a refusal, so a 200 or a 5xx recorded by mistake fails the script before the request
is made. A refused connection fails `curl -sS` itself. `read_status` stops on the
no-machine line, so each read while the machine runs is the positive control for the
no-machine lines. The `case` on `url` assumes the URL shape QS01 claim 5 recorded, and
the `lsof` and `awk` patterns the line shape it quoted; step 1 corrects either if it
differs. If the script stops early, run `just studio down` before running it again.
Steps 3 and 6 to 9 hold the other refusals, the maintainer's checks, the proof and the
reach probes, whose output is quoted there.

## Open points

Questions the design leaves for step 5's ruling. The builder proposes an answer to each
in `ai_tmp/qs04/plan.md` and decides none.

1. **The fallbacks, if step 3 fails.** For Studio under a read-only worktree: the other
   two ways QS01's "Go or no-go" lists beside it. For the data directory: another
   storage design, or a `down` that deletes Studio's data and says so. For a path that
   must be hidden and cannot be, and for a layout over the mount budget, there is no
   fallback written here: the maintainer rules.
2. **The data directory's path, and what removes it.** Whether it lies inside QS02's
   state directory, where QS02's removal recipe would delete the sign-in with the
   machine's state and QS02's marker is its identity record, or beside it, where it
   needs the removal `down` prints and a record of its own; and, for a record of its
   own, where it lies so that the guest cannot write it, and which id it holds if
   QS02's hand-back shows none that outlives its removal recipe. That the identity is
   checked is not open.
3. **The lock every worktree shares for port allocation.** Its path is outside any
   worktree, so it is one more thing written there, which decision 0022 must name and
   something must remove. It is one for all worktrees and is removed with none of them.
   QS02's own lock is another thing: QS02's hand-back says what its removal recipe
   leaves behind for a worktree, and the notes here name that as left, with the command
   that removes it.
4. **The empty directory mounted over the hidden paths.** Where it lives, and whether
   one serves every hidden path.
5. **The Studio install's own mount.** The read-only worktree already shows
   `.tools/quint-studio/` at its own path unless `.tools/` is hidden. Whether the
   install keeps a mount at a fixed guest path, which costs one against the budget.
6. **The names of the new files** under `tools/quint-studio/`: the wrapper, the
   provisioning step, and where the snapshot timestamp is recorded.
7. **What `down` loses with the machine's disk.** The objects the wrapper keeps there
   (ruling 7) go with the machine while Studio's projects stay in the data directory.
   Step 8 has the maintainer look; if a project does not read as before, the ruling is
   the maintainer's.
8. **A machine whose provisioning failed.** The design leaves it stopped and refuses it
   until `just studio down`, by its missing completion record. Whether `up` should
   instead delete a machine it has just made when its provisioning fails, as it does
   after a collision, so that the next `up` starts from nothing.
9. **Whose listener it is.** How `up` and `status` tell this machine's publisher from
   another process: by what smolvm itself reports for a machine, which step 2 confirms
   on the day. If smolvm reports no process for it, what the confirmation rests on
   instead; and whether `status` should print the publisher, so that Verification
   could match `lsof` against it.
10. **The record of a scan's git commands.** Whether the committed wrapper keeps a
    record of each call and its exit status always, only when asked, or never, with
    Studio's own logs as the source that steps 3 and 8 then quote.
11. **The host's address families.** If step 3's control finds the guest reaching the
    host in a family that the network settings upstream documents cannot deny, there is
    no fallback written here: the maintainer rules.

## Hand-back notes

Empty until the ticket runs. Sections, in order: Base and branch; Baseline; The facts and
the amendments; The proof; Ruling; Claims; What landed; Refusals, red and green; The
maintainer's checks; What is left outside the worktree, and its removal; Checkpoint 1
record; Checkpoint 2 record; Checkpoint 3 record; Departures; Verification; Open points.
