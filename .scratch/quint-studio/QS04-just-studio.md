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
machine state where decision 0022, `docs/decisions/0022-quint-studio-machine.md`, says.
After QS03, `just install-quint-studio` installs the Studio AppImage whose SHA-256
`tools/quint-studio/studio.txt` records. Nothing yet puts the two together. QS01,
`.scratch/quint-studio/QS01-studio-spike.md`, did that by hand, with flags typed at a
shell; this ticket turns its hand-back into a committed Smolfile and one recipe.

The maintainer's rulings of 2026-11-24, listed in QS01's Context, bind this ticket.
Ruling 2 sets the guest's access: the worktree read-write, open egress, host loopback and
private ranges blocked. Ruling 4 limits it to macOS on arm64. Port 7001, Studio's MCP
endpoint, stays closed here; QS05 opens it.

Read these:

- `.scratch/quint-studio/QS01-studio-spike.md`, its hand-back notes: "Facts for QS04",
  and the claims on the UI's answers, extraction, sign-in, git, persistence, egress,
  masking and machine control (claims 4 to 9, 13 and 16).
- `.scratch/quint-studio/QS02-pinned-smolvm.md` and
  `.scratch/quint-studio/QS03-pinned-studio.md`, their hand-back notes, for the rulings
  and the `--check` modes this recipe calls.
- `docs/decisions/0022-quint-studio-machine.md`.
- `Justfile`: `smolvm`, `install-smolvm`, `install-quint-studio`, `quint`.
- `docs/how-to/develop-locally.md`, "Optional: Quint Studio".
- `.agents/skills/review-docs/SKILL.md`, for the new page.
- `tickets/CONVENTIONS.md` §11, §13 and §14.

## Goal

`just studio up` boots Quint Studio's `serve` inside a machine built from the committed
Smolfile, and prints the UI's token URL on host loopback. `just studio status`, `stop`
and `down` report, stop and remove the machine. The recipe runs only over the stamped
smolvm and Studio installs, refuses a port that is taken, and keeps Studio's sign-in and
projects on a volume that survives `stop` and `up`. A new how-to page, "Run Quint
Studio", is the one place a contributor learns to use it.

The demo: `just studio up`; the maintainer signs in; Studio opens the worktree as a
project; after `just studio stop` and `just studio up`, it is still signed in with its
projects intact.

## Design proposed in this ticket

These are proposals. Step 1 amends them where QS01's hand-back or the QS02 and QS03
rulings disagree, and the maintainer rules on the result at step 4, with checkpoint 1's
findings in hand.

- **A committed Smolfile, `tools/quint-studio/Smolfile`**, holding what every worktree
  shares, in the keys upstream documents (QS01 "Facts for QS04"):
  - the guest image by digest, never by tag alone;
  - Rosetta on, with the CPU count and memory QS01 claim 11 found usable;
  - networking on, with private ranges and `127.0.0.0/8` denied, plus whatever QS01
    claim 9 found DNS needs;
  - Studio's data directory, and every other path QS01 claim 8 found it writes outside
    the worktree, on a data volume, whose name the Smolfile never writes literally;
  - the stamped `.tools/quint-studio/` read-only at a fixed guest path;
  - the command that extracts, or runs, the AppImage the way QS01 claim 4 found, and
    then runs `serve --bind 0.0.0.0`. The extracted tree never lives on the data volume,
    which outlives the machine; if it is kept between boots at all, its directory is
    named by the AppImage's SHA-256, so a changed AppImage is never served from an old
    extraction.

  What differs per worktree, the worktree's own path, the common git directory QS01
  claim 7 found necessary, the data volume's name (below) and the host ports,
  `scripts/studio.sh` passes as flags, if
  QS01 showed smolvm accepts flags beside a Smolfile. Otherwise it renders a copy of the
  Smolfile into the machine's state directory with those values filled in.
- **Guest packages.** If QS01 claim 4 found libraries the image lacks, the maintainer
  rules at step 4 between an image that already carries them, pinned by digest, and a
  provisioning step whose package sources are pinned to a dated snapshot recorded in the
  Smolfile. An unpinned `apt-get install` at boot is not a choice: it would make every
  boot install whatever the mirror holds that day.
- **The mounts.** The worktree read-write at its own host path, so paths in Studio, in
  git and in an agent on the host agree. The common git directory read-only at its own
  host path, or not at all, as QS01 claim 7 found: a guest that could write the shared
  git directory could plant hooks or configuration the host's git runs. The Studio
  install read-only.
- **Masks over the ignored paths.** The worktree mount exposes every gitignored path
  under it read-write, and no diff shows a change there. At step 1 the builder lists
  every pattern in `.gitignore` in `ai_tmp/qs04/amendments.md`, each with whether the
  host runs or loads what lies there. At least these are masked: `.tools/` and `.pixi/`,
  first on every recipe's `PATH`; `.quint/` and `tools/quint/node_modules/`, which
  `just quint` runs; `target/`, `fuzz/target/` and `loadgen/target/`, whose binaries the
  tests run; `.claude/` and `.codex/`, whose ignored local settings the agents load;
  `ai_tmp/`, where verification scripts are kept and run; and the machine state, with
  the data volume if it lives there. Each gets an empty or read-only mount over it, as a
  directory or a single file, the way QS01 claim 13 found works, except
  `.tools/quint-studio/`, which the guest needs read-only. `.claude/` and `.codex/`
  also hold tracked files, so theirs is the same path read-only. Before it creates or
  mounts anything, `scripts/studio.sh` refuses, exiting 1 and naming the recovery, a
  symlink or anything but a directory at each masked path and at each of its ancestors
  below the worktree root (`tools/`, `tools/quint/`, `fuzz/`, `loadgen/`, `.codex/` and
  the rest), as QS02's installer does for `.tools`; only then is a masked directory that
  is absent created empty, so that the mount has a place and the creation cannot follow
  a link out of the worktree.

  A nested mask protects only while its ancestors stay where the host expects them. If
  QS01 claim 13 found the guest can rename a writable ancestor of a nested mask, the
  design instead masks that ancestor read-only at the same path: `tools/` for
  `tools/quint/node_modules/`, and `fuzz/` and `loadgen/` for their `target/`. That
  makes tracked files under them read-only to the guest, a narrowing of ruling 2's
  read-write mount, so the maintainer rules on it at step 4.

  If claim 13 failed for any of them, the builder does not choose: step 4 stops for the
  maintainer's ruling, unless decision 0022 already records one from QS02. Moving the
  machine state out of the worktree keeps ruling 2 but protects only the state; the
  other paths stay exposed. Dropping the read-write mount changes ruling 2. Accepting
  the exposure changes this ticket's acceptance criterion on masked writes. Either of
  the last two needs the maintainer to amend the ruling or the criterion, and the
  hand-back and decision 0022 record the amendment and name each unmasked path, as the
  page does.
- **`scripts/studio.sh`, run by `just studio <action>`**, POSIX `sh`:
  - `up`: refuses any host but macOS on arm64; runs `scripts/install-smolvm.sh --check`
    and `scripts/install-quint-studio.sh --check`, failing and naming
    `just install-smolvm` or `just install-quint-studio`. It then computes the machine's
    input digest: the SHA-256 of the smolvm stamp's `archive=`, the Studio stamp's
    `sha256=` and `version=`, the Smolfile's SHA-256 and every per-worktree value it
    passes. A machine that exists whose recorded digest differs, or that has none, is
    stopped first if it runs, keeping its volume, so that nothing it serves outlives the
    refusal; then `up` refuses, naming `just studio down` and then `just studio up`,
    which rebuilds the machine and keeps the data volume. A machine already running with
    the same digest is reported, not restarted, and the port check is skipped for it,
    since the listener is its own. Otherwise `up` checks the port (below), creates the
    machine if it is absent, recording the digest beside the machine's state where QS01
    claim 16 found smolvm leaves it alone, and starts it with no `serve` running; reads
    `quint-studio --version` in the guest, the way claim 16 found a command runs before
    `serve`, and stops the machine and refuses if it is not the pin's `version=`; only
    then starts `serve`, waits for the UI, failing closed on a timeout with the guest's
    last log lines, and prints the token URL. Every refusal exits 1 with nothing of this
    worktree's served when it returns, and a message naming what to run; all but the
    version and digest refusals also come before anything boots.
  - `status`: whether the machine exists and runs, its ports, Studio's version, and the
    token URL when it runs.
  - `stop`: stops the machine and keeps it and its volume.
  - `down`: deletes the machine and keeps its data volume. It prints the command that
    deletes the volume, and says that doing so deletes the sign-in and every Studio
    project; the recipe never runs it.
  - Any other action, or none, prints the four and exits non-zero.
- **The machine's name** is `pachinko-studio-<worktree>-<hash>`: the worktree's
  directory name lowercased, every character outside `a-z`, `0-9` and `-` replaced by
  `-`, and the first eight hex digits of the SHA-256 of the worktree's absolute path. Its
  data volume is that name with `-data` appended, named the way QS01 claim 16 found, and
  `scripts/studio.sh` passes it per worktree, so two worktrees never share a machine or
  a volume.
- **Ports.** The UI is published on `127.0.0.1:${STUDIO_UI_PORT:-7000}`, never on any
  other host address. `STUDIO_UI_PORT` must be a decimal integer from 1024 to 65535.
  Before `up` boots anything, `lsof -nP -iTCP:<port> -sTCP:LISTEN` must find no
  listener; otherwise it refuses, naming the process and `STUDIO_UI_PORT`. A failure of
  `lsof` itself is a refusal, not a free port.
- **The token** is read the way QS01 claim 5 found and printed to the terminal only. It
  is never written to a file the worktree's Git can see.
- **Writes inside the worktree.** Every path QS01 claim 8 found Studio writing inside a
  project directory is either turned off through Studio's settings or ignored in
  `.gitignore`, with a comment; the ruling at step 4 picks. `just check` snapshots the
  worktree, so a path Studio writes that Git can see would fail it.
- **`just studio` reaches the network**: the first `up` pulls the image, and the guest
  has open egress. `AGENTS.md` lists it.
- **The page, `docs/how-to/run-quint-studio.md`**, registered in `docs/manifest.yml`
  as the owner of the topic and reachable from `docs/README.md`. It covers the two
  installs, `up`, signing in, opening the worktree, `stop`, `down`, the port override,
  deleting the volume and what that loses, and what the guest can reach and write, citing
  decision 0022. `docs/how-to/develop-locally.md`, "Optional: Quint Studio", shrinks to a
  pointer at it.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 1 | The machine built from the Smolfile serves the UI on host loopback only | step 6, `curl` and `lsof` |
| 2 | Studio's sign-in and projects survive `stop` and `up`, and `down` then `up` | step 7, the maintainer looking |
| 3 | From the guest, host loopback and the host's LAN address are unreachable, and a public HTTPS host is reachable | step 8, QS01 step 10's listeners, probed at the guest-side address QS01's control boot found reaching host loopback |
| 7 | From the guest, a write under each masked path of the design, as step 1's list has it, fails, and the host's copy is unchanged, and renaming a writable ancestor of a nested mask does not move the host's path off the masked tree; or, if step 4's ruling amended this, each path the ruling left unmasked is named in decision 0022 and on the page | step 8, one write attempt per path |
| 4 | A Studio session changes nothing Git sees in the worktree: `git status --porcelain --untracked-files=all`, `git diff HEAD`, and the SHA-256 of every untracked file `git ls-files -o --exclude-standard` lists are the same before and after it | step 8, `cmp` on each pair |
| 5 | A file the guest writes in the worktree is owned on the host by the user who ran `just studio`, with a mode no wider than the guest asked for | step 8, `ls -ln` |
| 6 | Two worktrees get two machines and two volumes, and data one guest writes to its volume is absent from the other's | step 6, a second worktree booted |
| 8 | `up` refuses an existing machine whose recorded input digest differs from the one it computes, stopping it first if it runs, and reads the guest's `--version` before `serve` starts | step 5 |

## Non-goals

- No MCP. Port 7001 is not published; QS05 publishes it.
- No CI job: no runner offers the hypervisor (decision 0022).
- No gate that runs Studio. `just check` neither needs nor touches the machine.
- No Linux host.
- No `CHANGELOG.md` entry.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review" (T37 is
still todo). `<id>` is `QS04`. There are two checkpoints, at steps 3 and 10.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tools/quint-studio/Smolfile` | new | the machine, with a comment naming decision 0022 |
| `scripts/studio.sh` | new | `up`, `status`, `stop` and `down`, POSIX `sh` |
| `docs/how-to/run-quint-studio.md` | new | the how-to page |
| `Justfile` | shared, own lines | `studio *args`, with a comment |
| `.gitignore` | shared, own lines | only if the ruling ignores paths Studio writes in the worktree |
| `AGENTS.md` | shared, own lines | `just studio` in the list of recipes that reach the network |
| `docs/manifest.yml` | shared, own lines | the page's entry |
| `docs/README.md` | shared, own lines | a link to the page |
| `docs/decisions/0022-quint-studio-machine.md` | shared, own lines | Consequences: the data volume holds a credential, where it lives, and what `down` keeps; the masked paths, and, if step 4's ruling amended the masks, that ruling and each path left unmasked |
| `docs/how-to/develop-locally.md` | shared, own lines | "Optional: Quint Studio" becomes a pointer at the new page |
| `docs/reference/commands.md` | shared, own lines | a `studio` row and the network list |
| `docs/reference/configuration.md` | shared, own lines | `STUDIO_UI_PORT`, and a row for the Smolfile |
| `docs/explanation/security-model.md` | shared, own lines | the guest's reach and writes, and the token |
| `docs/project/repository-map.md` | shared, own lines | the Smolfile and `scripts/studio.sh` |
| `.scratch/quint-studio/QS04-just-studio.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS04's Status cell |

## Steps

Every quoted URL has its token replaced by `<redacted>`.

1. **Worktree, baseline and the facts.** Create the worktree on
   `ticket/qs04-just-studio` from a base that holds QS02 and QS03, and record the base
   and its full commit id. Run `just initialize` (network, authorised),
   `just install-smolvm` and `just install-quint-studio` (network, authorised), and
   `just check` on the untouched worktree, and quote its last line. If
   `just install-quint-studio` fails on a hash mismatch, upstream has moved: **stop** for
   the maintainer's re-pin, by the procedure QS03 wrote. Read the hand-backs QS04 cites
   and write `ai_tmp/qs04/amendments.md`, as QS02's step 1 does, with the list of every
   `.gitignore` pattern and whether the host runs or loads what lies there, which the
   masks follow.
2. **Read before writing.** Confirm on the day, with `just smolvm --help` and the help of
   each subcommand used, every flag and Smolfile key the design needs. Write
   `ai_tmp/qs04/plan.md`: the Smolfile's content, `scripts/studio.sh`'s outline against
   "Design proposed" as amended, the edit to each page in Files touched, and the path
   proposed for step 6's second worktree with the commands that create and remove it.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qs04/plan.md` and `ai_tmp/qs04/amendments.md`, before any tracked file
   changes). Questions:
   - Can `up` boot without both stamped installs, or with a Studio whose version is not
     the pin's?
   - Can the UI port be bound on an address other than `127.0.0.1`?
   - Can the guest reach host loopback, a private range, or the common git directory
     with write access?
   - Can a port check that fails, or a wait that times out, read as success?
   - Can `down` lose the sign-in, or can any action delete the volume?
   - Can a path Studio writes make `just check` fail?
   - Does the mask list miss an ignored path the host runs or loads?
   - Can an existing machine serve a Studio, a Smolfile or a mount other than the ones
     `up` checked, or can two worktrees share a volume?
4. **The ruling.** **Stop** for the maintainer's ruling on "Design proposed" as amended,
   with the checkpoint's findings in hand, including the guest packages, the paths
   Studio writes in the worktree and, if QS01 claim 13 failed for any path, the masks,
   and record it. The same stop asks for the path of step 6's second worktree, which
   lies outside this one, and for leave to create and remove it there; checkpoint 1's
   brief names that request, so no other review is needed before it.
5. **The machine and the recipe.** Add the Smolfile, `scripts/studio.sh` and
   `just studio`. Show, quoting each run and its exit status:
   - with `.tools/smolvm/.pachinko-stamp` moved aside, `just studio up` exits 1 naming
     `just install-smolvm`; with `.tools/quint-studio/.pachinko-stamp` moved aside, it
     exits 1 naming `just install-quint-studio`; each boots nothing (`just studio status`
     prints the line it gives for no machine, quoted once as the control), and each
     stamp is restored afterwards (`cmp` exits 0);
   - with a listener on `127.0.0.1:7000` (`python -m http.server --bind 127.0.0.1 7000`
     from the pixi environment), `just studio up` exits 1, naming the process and
     `STUDIO_UI_PORT`, and boots nothing;
   - `STUDIO_UI_PORT=80`, `STUDIO_UI_PORT=70000` and `STUDIO_UI_PORT=abc` each exit 1,
     naming `STUDIO_UI_PORT`, and boot nothing;
   - claim 8, after one `up` and `stop` (network, authorised): the recorded digest
     edited by one character, then removed, and the Smolfile changed by one comment
     byte, each restored afterwards: `just studio up` exits 1 naming `just studio down`,
     with the machine stopped and with it running; in the running case `up` stops it
     first, and afterwards `just studio status` reports it stopped and
     `lsof -nP -iTCP:7000 -sTCP:LISTEN` finds no listener. Restored, `up` starts it. The
     guest's log shows `--version` read before `serve` started;
   - with `fuzz/target` absent and replaced by a symlink to a new, empty directory under
     `ai_tmp/qs04/`, `just studio up` exits 1 naming the recovery, boots nothing, and
     the target stays empty; restored afterwards;
   - `just studio` alone and `just studio restart` each print the four actions and exit
     non-zero.
6. **Up** (network, authorised: the image pull and the guest's egress). `just studio up`
   prints the token URL. From the host, `curl` of the UI with the token answers 200 and
   without it gives exactly the status, and any redirect target, QS01 claim 5 recorded;
   `lsof -nP -iTCP:7000 -sTCP:LISTEN` shows a listener on `127.0.0.1` only (claim 1).
   Claim 6: from this guest, write a file holding a random value to the data volume.
   The second worktree needs the implementation under test, so first commit the work
   so far on the ticket branch (step 11's commit builds on it). Create the second
   worktree at the path step 4's ruling named, with
   `git worktree add --detach <path> HEAD`, show `git -C <path> rev-parse HEAD` equal to
   this worktree's, and in it run `just initialize`,
   `just install-smolvm` and `just install-quint-studio` (network, authorised).
   `just studio status` there reports no machine; then
   `STUDIO_UI_PORT=7100 just studio up` there (network, authorised: its own image pull
   and egress) boots its own machine. Show the two machine names and the two volume
   names differing, in the volume listing QS01 claim 16 recorded, and the file absent
   from the second guest's volume. Then, in the second worktree, `just studio down` and
   the volume deletion `down` printed, which loses no sign-in; then
   `git worktree remove <path>` from this one, and `test ! -e <path>`; quote each.
7. **Sign-in and persistence.** **Stop for the maintainer**, who opens the URL, signs
   in, configures the AI provider and opens this worktree as a project. Then
   `just studio stop`, `just studio up`: the maintainer confirms Studio is still signed
   in with the project listed. Then `just studio down`, `just studio up`: the same
   (claim 2). Record nothing the maintainer typed.
8. **Reach and writes.** Repeat QS01 step 10's listeners and public request from inside
   the running machine (claim 3). Claim 4: later edits may not yet be committed, so the
   check compares rather than asks for an empty status. Before a Studio session in which
   the maintainer opens a file and runs one action, save to `ai_tmp/qs04/`
   `git status --porcelain --untracked-files=all`, `git diff HEAD`, and `shasum -a 256`
   of every untracked file `git ls-files -o --exclude-standard` lists, which covers the
   new Smolfile, script and page that `git diff HEAD` does not; after it, save all three
   again, and show `cmp` exiting 0 on each pair. The empty status after the commit is
   Verification's. Have the guest create `qs04-owner-probe` at the worktree root
   (`ai_tmp/` is masked), show `ls -ln` of it on the host (claim 5), and remove it. From
   the guest, try one write under each masked path, and show each failing and the host's
   copy unchanged; then try to rename each writable ancestor of a nested mask, as QS01
   step 9 did, and show the host's path still leading to the masked tree (claim 7). Run
   `just check` while Studio runs, and quote its last line.
9. **Words.** Write the page and edit each page in Files touched, following
   `.agents/skills/review-docs/SKILL.md`. Run `just check-docs` and `just check-agents`.
10. **Adversarial review** (checkpoint 2: conformance and breakage, over the diff with
    the pages included, before the final commit). Questions:
    - Could Verification pass while the UI was published beyond host loopback, or while
      Studio's version differed from the pin?
    - Does any committed file, quoted output or page hold a token?
    - Does `just check` stay free of the network and of the machine?
    - Does the page say, plainly, what the guest can write and reach, and what deleting
      the volume loses?
11. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed by
    the full commit id. Run `just studio down`. Set `status: done` here and in
    `.scratch/quint-studio/README.md`, write the hand-back notes, and commit the record.
    On the record commit, run `git diff --name-only HEAD~1 HEAD`, which must list only
    this ticket and the README, then `just check` and `git status --short`, and report
    their output to the maintainer with the hand-back, headed by that commit, since a
    commit cannot quote its own check. If any fails, fix the cause in a new commit and
    run all three again. **Stop** before pushing or opening a pull request: both need
    authorisation (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `just studio up` refuses without either stamped install, on a taken port, on a
  malformed `STUDIO_UI_PORT`, on a symlink at a masked path or its ancestor, and over a
  machine whose input digest differs, stopping that machine if it runs, each with exit
  1, its diagnostic and nothing of this worktree's served afterwards, and boots
  otherwise, reading the
  guest's `--version` before `serve` and printing the token URL, each quoted with the
  token redacted.
- The UI answers on `127.0.0.1` only, with 200 only with the token, and without it
  exactly the refusal QS01 claim 5 recorded: 401 or 403, or a redirect to `127.0.0.1`.
- Two worktrees get two machines and two volumes, and one guest's volume data is absent
  from the other's, quoted.
- Studio's sign-in and projects survive `stop` and `up`, and `down` and `up`, as the
  maintainer confirms.
- The guest cannot reach host loopback or the host's LAN address, and can reach a public
  HTTPS host, each quoted.
- From the guest, a write under each masked path fails, each quoted. If step 4's ruling
  amended this criterion after QS01 claim 13 failed, the ruling is quoted instead, and
  decision 0022 and the page name each path it left unmasked.
- A Studio session changes nothing Git sees in the worktree, by step 8's comparison.
- `docs/how-to/run-quint-studio.md` is registered and reachable, and `just check-docs`
  passes.
- `AGENTS.md` lists `just studio` among the recipes that reach the network, and
  `just check-agents` passes.
- Each claim has its outcome recorded.
- Both checkpoints are recorded as §14's Record asks, with AP01's deltas.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and again after the record
  commit, as step 11 reports.

## Verification

Run from a file as one `sh` script after the commit, with the machine down; `set -eu`
stops it at the first command that fails. It reaches the network (the guest's egress),
under the ticket's authorisation. No pipeline carries a result: each command writes to
a file, its exit status is checked, and the file is matched exactly.

```sh
set -eu
git rev-parse HEAD
no_machine='<the line just studio status prints for no machine, quoted at step 5>'
refused='<the HTTP status QS01 claim 5 recorded for a request without the token>'
location='<the redirect target QS01 claim 5 recorded for it, or empty if none>'
stamp=.tools/quint-studio/.pachinko-stamp
out=ai_tmp/qs04/verify
mkdir -p "$out"
restore() {
    if [ -e "$out/studio-stamp" ]; then mv "$out/studio-stamp" "$stamp"; fi
}
trap restore EXIT

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

# Green: up serves on host loopback only, with 200 only with the token.
just studio up >| "$out/up.log" 2>&1
port=${STUDIO_UI_PORT:-7000}
lsof -nP -iTCP:"$port" -sTCP:LISTEN >| "$out/lsof.txt"
grep -F "127.0.0.1:$port (LISTEN)" "$out/lsof.txt"
rc=0; grep -v -F "127.0.0.1:$port (LISTEN)" "$out/lsof.txt" >| "$out/lsof-other.txt" || rc=$?
test "$rc" -le 1
if grep -F '(LISTEN)' "$out/lsof-other.txt"; then exit 1; else test "$?" -eq 1; fi
grep -o "http://127.0.0.1:$port/?token=[^[:space:]]*" "$out/up.log" >| "$out/urls.txt"
url=$(sed -n 1p "$out/urls.txt")
test -n "$url"
code=$(curl -sS -o /dev/null -w '%{http_code}' "$url")
test "$code" = 200
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
answer=$(curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' "http://127.0.0.1:$port/")
test "$answer" = "$refused $location"
just studio status >| "$out/status-up.txt" 2>&1
if grep -Fx "$no_machine" "$out/status-up.txt"; then exit 1; else test "$?" -eq 1; fi
just studio stop
just studio up >| "$out/up-again.log" 2>&1
just studio down
just studio status >| "$out/status-after.txt" 2>&1
grep -Fx "$no_machine" "$out/status-after.txt"

just check
status=$(git status --short)
test -z "$status"
```

The three placeholders are filled in from step 5's quote and QS01's hand-back before the
script runs. The files under `ai_tmp/qs04/verify/` that hold the token, `up.log`,
`up-again.log`, `urls.txt` and `status-up.txt`, are never quoted unredacted, and `url`
is never echoed. The first `lsof` `grep` is the positive control for the silent probe
after it: both read the same output, and `grep -v` failing with status 2 stops the
script. The request with the token is the positive control for the one without it, which
must give exactly the recorded answer. The `case` checks that recorded answer
independently: only 401, 403, or a redirect to a path on the same loopback address other
than the bare UI counts as a refusal, so a 200 or a 5xx recorded by mistake fails the
script before the request is made. A refused connection fails `curl -sS` itself. The
status line read while the machine runs is the positive control for the no-machine
lines. The `grep -o` pattern assumes the URL shape QS01 claim 5 recorded; step 1
corrects it if the shape differs. If the script stops early, run `just studio down`
before running it again. Steps 5 to 8 hold the other refusals, the maintainer's checks
and the reach probes, whose output is quoted there.

## Hand-back notes

Empty until the ticket runs. Sections, in order: Base and branch; Baseline; The facts and
the amendments; Ruling; Claims; What landed; Refusals, red and green; The maintainer's
checks; Checkpoint 1 record; Checkpoint 2 record; Departures; Verification; Open points.
