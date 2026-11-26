---
id: QS03
title: "Quint Studio's AppImage pinned by its observed SHA-256"
status: todo
depends_on: [QS01]
parallel_with: [QS02]
branch: ticket/qs03-pinned-studio
estimated_size: S
---

# QS03: Quint Studio's AppImage pinned by its observed SHA-256

## Context

QS01, `.scratch/quint-studio/QS01-studio-spike.md`, ran Quint Studio's Linux AppImage
inside a smolvm guest by hand. Upstream offers that AppImage only as `stable/latest`:
the URL names no version, the bytes behind it change with every release, and no checksum
is published. Decision 0021, `docs/decisions/0021-project-managed-quint-cli.md`,
concluded from this that nothing about Studio could be pinned.

The maintainer ruled otherwise on 2026-11-24 (ruling 1, stated in the README of these
tickets, `.scratch/quint-studio/README.md`): Studio is pinned by the SHA-256 of the
build the maintainer reviewed. The installer downloads `stable/latest` and installs it
only if its hash is the pinned one. When upstream ships a new build, the install fails
until someone reviews and re-pins it. That detects drift; it does not make installs
reproducible, because the installer uses only the moving `stable/latest` URL and cannot
rely on fetching an older build once that URL has moved. The other rulings bind this
ticket as that README states them: ruling 4 limits it to macOS on arm64, and ruling 9
has decision 0022 and the handbook say what the pin does not cover at run time.

The AppImage is a Linux x86_64 executable, so the macOS host cannot run it, and by QS01
claim 12 the build prints no version in the guest either. The installer therefore checks
only the hash. The version the pin file records is taken the way QS01 claim 12 and
"Facts for QS03" record it, beside the hash, and nothing reads it back from the build.
What QS04's `just studio` checks in the guest, before it serves, is that the extracted
tree comes from the pinned AppImage, by its SHA-256; "Facts for QS03" says why. This
differs from a first sketch in which the installer, and later the guest, read
`--version`.

QS03 is built in parallel with QS02, but it lands after QS02. Both tickets edit the same
pages and gate 2's recipe, and QS03's words cite decision 0022, which QS02 writes. So
QS03 rebases onto a base holding QS02 before step 8, and reruns its verification there.

Read these:

- `.scratch/quint-studio/README.md`, "The maintainer's rulings".
- `.scratch/quint-studio/QS01-studio-spike.md`, its hand-back notes: claim 12, "Facts
  for QS03" and Open point 7.
- `.scratch/apalache/AP02-checksummed-apalache.md`, "Design proposed in this ticket" and
  "Steps", for the pin file, the stamp and the gate's red tests.
- `.scratch/apalache/AP01-java-and-verify.md`, "Adversarial review".
- `.scratch/quint-studio/QS02-pinned-smolvm.md`, "Design proposed in this ticket", for
  the shape its installer shares with this one.
- `Justfile`: `install-apalache` and `lock-check`. `scripts/install-apalache.sh`.
- `tickets/CONVENTIONS.md` §11, §13 and §14.

## Goal

`just install-quint-studio` downloads `stable/latest` and installs it into
`.tools/quint-studio/` only if its SHA-256 equals the one `tools/quint-studio/studio.txt`
records. On a mismatch it prints both hashes and the re-pin procedure, and installs
nothing. A rerun over a stamped install skips and reaches no network. Gate 2,
`just lock-check`, fails offline on a malformed pin file.

What stays open, and the pages say so: the pin detects drift, and the installer, which
uses only the moving `stable/latest` URL, cannot rely on fetching an older build once
that URL has moved; the version the pin file names is recorded, not checked against the
build, which prints none (QS01 claim 12); and the pin does not cover what Studio does at
run time, which QS01's hand-back lists as its Open point 7 (ruling 9).

## Design proposed in this ticket

These are proposals. Step 1 amends them where QS01's hand-back disagrees, and the
maintainer rules on the result at step 4, with checkpoint 1's findings in hand.

- **One pin file, `tools/quint-studio/studio.txt`.** Comment lines start with `#`. Four
  `key=value` lines follow: `url=` the `stable/latest` URL QS01 recorded; `version=` the
  version string of this build, taken the way QS01 claim 12 and "Facts for QS03" record
  it, since the build prints none; `sha256=` the AppImage's hash, lowercase hex; and
  `reviewed=` the date of the maintainer's review, `YYYY-MM-DD`. The scripts read it
  with `sed` and never source it.
- **`scripts/check-quint-studio-pin.sh`, in `lock-check`.** Offline and read-only,
  reading only committed files. It fails unless the file has exactly the four keys,
  `url` starts with `https://` and holds no whitespace, `version` is non-empty and
  matches the pattern QS01's string fits, `sha256` is 64 lowercase hex digits, and
  `reviewed` is a well-formed date. It never compares the date with the clock, so the
  gate's answer does not change from one day to the next. Each failure exits 1 and
  prints a line of the form `tools/quint-studio/studio.txt: <key>: <reason>`, which
  Verification matches.
- **`scripts/install-quint-studio.sh`, run by `just install-quint-studio`.** In order:
  1. It refuses any host but macOS on arm64, naming ruling 4 and decision 0022.
  2. It fails if `scripts/check-quint-studio-pin.sh` would.
  3. It refuses a symlink, or anything but a directory, at `.tools` or
     `.tools/quint-studio`, and a symlink, or anything but a regular file, at the
     AppImage's path and at the stamp's path where one exists, naming the recovery, as
     `scripts/install-apalache.sh` does for its own paths. Each test is made without
     following a link, since a plain regular-file test follows one. Without that a
     symlinked `.tools` would carry the download outside the worktree, a symlinked
     `.tools/quint-studio` would give QS04's read-only mount a tree this installer never
     checked, and a symlink at the AppImage's path, to a file elsewhere that has the
     pinned hash and mode, would pass the skip test below.
  4. It skips, and says so, only if `.tools/quint-studio/.pachinko-stamp` exists, its
     `sha256=` and `version=` are the pinned ones, the AppImage's SHA-256 recomputed now
     is the pinned one, and its mode, read with `stat -f '%Lp'`, is `555`.
  5. It downloads `url` with `curl -fsSL --proto '=https' --proto-redir '=https'` into a
     new temporary directory under `.tools/` (from `mktemp -d`), reads the final URL
     the download reached the way "Facts for QS03" records, and computes the SHA-256
     with `shasum -a 256`.
  6. On a mismatch it prints the pinned hash, the downloaded hash, the final URL
     `curl` reached, and the re-pin procedure's location,
     `docs/how-to/maintain-dependencies.md`, "Quint Studio". It removes the temporary
     directory, leaves any existing install untouched, and exits non-zero.
  7. On a match it names the file `quint-studio.AppImage`, sets its mode to `0555`,
     writes the stamp beside it, and only then replaces `.tools/quint-studio/` with the
     temporary directory. The install is then `.tools/quint-studio/quint-studio.AppImage`
     and `.tools/quint-studio/.pachinko-stamp`, nothing else.
  8. Every `rm -rf` names its path through `"${var:?}"`, and a trap removes the
     temporary directory on every exit.
- **`--check` mode** for QS04: offline and read-only, checks 1 to 3 and step 4's skip
  test, exit 0 only when that test would skip. Otherwise it exits 1 and prints a message
  naming `just install-quint-studio`.
- **Not in `just initialize`.** Once upstream moves `stable/latest`, every install fails
  until the re-pin. If `initialize` ran this installer, every new worktree's first run
  would fail on a tool that most worktrees never use. `just studio` (QS04) names
  `just install-quint-studio` when the stamped install is missing. The alternative, in
  `initialize` with its failure tolerated, is in Open points for the ruling.
- **The re-pin procedure**, a subsection "Quint Studio" in
  `docs/how-to/maintain-dependencies.md`: download the new build into a new, empty
  `ai_tmp/` directory; compute its hash; take its version the way QS01 claim 12 and
  "Facts for QS03" record it, since the build prints none; review what changed
  upstream; edit `version=`, `sha256=` and `reviewed=` together; run
  `just install-quint-studio`. No Renovate manager reads the pin file.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 1 | `stable/latest` still serves the build QS01 hashed, or the maintainer reviews the newer one before it is pinned | step 2 |
| 2 | `curl` with `--proto-redir '=https'` follows every redirect `stable/latest` takes | step 2, the request QS01's "Facts for QS03" records as following them, with the flags |
| 3 | `just install-quint-studio` over a stamped install reaches no network | step 7, under §11's denial with its negative controls: exit 0 together with the installer's own skip message |

## Non-goals

- No run of Studio. QS04 boots it, and checks in the guest that the extracted tree
  comes from the pinned AppImage. Nothing checks `version=` against the build, which
  prints none (QS01 claim 12).
- No archive of old builds, and no download from any URL but `stable/latest`. Ruling 1
  accepts that installs are not reproducible. Whether an older build can still be
  fetched some other way is neither relied on nor denied here: "Facts for QS03" records
  what QS01 saw of the final URL on the day, and nothing shows how long that holds.
- No Renovate manager for the pin file.
- No `CHANGELOG.md` entry.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review" (T37 is
still todo). One delta overrides them: the reviewer is Codex Sol, `-m gpt-6.2-sol`, in
place of the `-m gpt-6-astra` that AP01's command and §14 print, by the maintainer's
choice of 2026-11-25. AP01's fallback to a fresh-context `claude-opus-5-5` subagent
still applies, and the hand-back notes record the model that ran. `<id>` is `QS03`.
There are two checkpoints, at steps 3 and 9.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `tools/quint-studio/studio.txt` | new | the pin file, with a comment naming decision 0022 and the re-pin procedure |
| `scripts/install-quint-studio.sh` | new | the installer, POSIX `sh`, with `--check` |
| `scripts/check-quint-studio-pin.sh` | new | the gate's check, POSIX `sh` |
| `Justfile` | shared, own lines | `install-quint-studio`, with a comment; a line in `lock-check` and its comment |
| `AGENTS.md` | shared, own lines | `just install-quint-studio` in the list of recipes that reach the network |
| `docs/decisions/0022-quint-studio-machine.md` | shared, own lines | the Studio pin: its file, its keys, where the version comes from and that nothing checks it against the build, the limit of ruling 1, and what the pin does not cover at run time (ruling 9), as QS01's Open point 7 lists it |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | the subsection "Quint Studio"; gate 2's description |
| `docs/reference/commands.md` | shared, own lines | an `install-quint-studio` row, the network list and the `lock-check` row; the `initialize` row only if the step 4 ruling puts the installer in `just initialize` |
| `scripts/initialize.sh` | shared, own lines | only if the step 4 ruling puts the installer in `just initialize`: `just install-quint-studio` after `just install-smolvm`, or after `just install-apalache` if QS02's ruling left smolvm out, on macOS arm64 only, its failure reported and tolerated, with a comment naming decision 0022 |
| `docs/how-to/develop-locally.md` | shared, own lines | only if the step 4 ruling puts the installer in `just initialize`: what `just initialize` installs, and that a stale pin's failure there is tolerated |
| `docs/tutorials/first-change.md` | shared, own lines | only if the step 4 ruling puts the installer in `just initialize`: its list of what `just initialize` installs |
| `docs/reference/configuration.md` | shared, own lines | a row for `tools/quint-studio/studio.txt` and the pins table |
| `docs/reference/quality-gates.md` | shared, own lines | gate 2's row and the recipes that reach the network |
| `docs/explanation/security-model.md` | shared, own lines | Studio checked by its observed hash, the limit of that check, and what the pin does not cover at run time (ruling 9), as QS01's Open point 7 lists it |
| `docs/project/repository-map.md` | shared, own lines | `tools/quint-studio/` and the two scripts |
| `.scratch/quint-studio/QS03-pinned-studio.md` | ticket | `status:`, the hand-back notes |
| `.scratch/quint-studio/README.md` | shared, own lines | QS03's Status cell |

## Steps

1. **Worktree, baseline and QS01's facts.** Create the worktree on
   `ticket/qs03-pinned-studio` from the base the maintainer names, and record the base
   and its full commit id. Check that it holds both QS01's hand-back, with its "Rulings
   after the hand-back" section, and `.scratch/quint-studio/README.md` stating rulings 1
   to 10; if either is missing, **stop** and report. Run `just initialize` (network,
   authorised) and `just check` on the untouched worktree, and quote its last line. Read
   QS01's claim 12, "Facts for QS03" and Open point 7, and write
   `ai_tmp/qs03/amendments.md` as QS02's step 1 does. The amendments carry the
   particulars this ticket leaves to them: how the version is taken, the pattern
   `version` must match, and how the redirects and the final URL are read.
2. **Read before writing** (network, authorised). Read the redirects the
   `stable/latest` URL takes with the request QS01's "Facts for QS03" records as
   following them, which the `curl -fsSIL` of this ticket's first draft is not, with the
   installer's flags, and record each redirect and the final URL (claim 2). Download the
   AppImage into a new, empty `ai_tmp/qs03/download/` and compute its SHA-256. If it
   differs from QS01's, a new build has shipped: **stop** for the maintainer, who
   reviews it before it is pinned, with its version taken the way QS01 claim 12 and
   "Facts for QS03" record it (claim 1). Never copy a hash from a page or a chat. Write
   `ai_tmp/qs03/plan.md`: the pin file's content, each script's outline against "Design
   proposed" as amended, and the edit to each page in Files touched.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/qs03/plan.md` and `ai_tmp/qs03/amendments.md`, before any tracked file
   changes). Questions:
   - Can the installer leave, or skip over, an AppImage whose hash is not the pin's?
   - Can a mismatch damage an existing install, or leave a temporary directory behind?
   - Could a redirect downgrade the download to plain HTTP?
   - Can a symlinked `.tools` or `.tools/quint-studio` carry the download or the install
     outside the worktree, or give QS04 a tree this installer never checked? Can a
     symlink at the AppImage's path or the stamp's pass the skip test?
   - Does any page claim the pin makes installs reproducible, that an older build can
     never be fetched, that anything reads a version from the build, or that the pin
     covers what Studio does at run time (ruling 9)?
4. **Red: the gate.** **Stop** first for the maintainer's ruling on "Design proposed" as
   amended, with the checkpoint's findings in hand, and record it. Then add the pin file,
   `scripts/check-quint-studio-pin.sh` and the `lock-check` line. Save the pin file to
   `ai_tmp/qs03/studio.txt.saved`. Show `just lock-check` passing. Then, each on an edit
   restored from the saved copy afterwards, show it failing and quote each message:
   `url=http://` in place of `https://`; the `version=` line removed; `sha256=` given 63
   digits; `reviewed=2026-13-01`; a fifth key added. After the last restore,
   `cmp tools/quint-studio/studio.txt ai_tmp/qs03/studio.txt.saved` exits 0.
5. **The installer.** Add `scripts/install-quint-studio.sh`,
   `just install-quint-studio`, and, if the step 4 ruling puts the installer in
   `just initialize`, the line in `scripts/initialize.sh`. Then, quoting each run and
   its exit status and restoring each edit afterwards:
   - with no `.tools/quint-studio/` there (`test ! -e`, after removing one an earlier
     run left), `just install-quint-studio` (network, authorised) installs and writes
     the stamp;
   - a second run skips and says so;
   - one byte appended to the installed AppImage, with `chmod u+w` before the append and
     `chmod 0555` after it, so that the mode reads `555` again (`stat -f '%Lp'`, quoted)
     and only the hash differs: the installer replaces it (network, authorised);
   - the stamp's `version=` edited: the installer replaces the install (network,
     authorised);
   - `chmod u+w` on the AppImage: the installer replaces it (network, authorised);
   - the installed AppImage moved to `ai_tmp/qs03/` and a symlink to it left at its
     path, the target still the pinned bytes at mode `0555`, and separately the same
     with the stamp: the installer and `--check` each refuse, exiting 1 and naming the
     recovery, and the target is unchanged (its SHA-256 and `ls -la` before and after);
   - `sha256=` changed by one digit, with the stamped install in place: the installer
     (network, authorised) fails, prints both hashes, the final URL and the re-pin
     procedure's location, leaves the existing install byte for byte as it was
     (`cmp` against a copy saved to `ai_tmp/qs03/` first), and leaves no temporary
     directory;
   - the same with `.tools/quint-studio/` removed first: it fails the same way and
     leaves no `.tools/quint-studio/`;
   - `.tools/quint-studio` replaced by a symlink to a new, empty directory under
     `ai_tmp/qs03/`, and separately `.tools` moved aside and replaced by a symlink to a
     copy of it under `ai_tmp/qs03/`: the installer and `--check` each refuse, exiting 1
     and naming the recovery, and the symlink's target is unchanged (`ls -la` before and
     after);
   - the host check, with `uname` shadowed as in QS02's installer step: it refuses;
   - the pin restored and `just install-quint-studio` run once more (network,
     authorised);
   - only if the step 4 ruling puts the installer in `just initialize`, with the stamped
     install in place: `just initialize` (network, authorised) runs the installer, which
     skips, and exits 0; then, with `sha256=` changed by one digit, `just initialize`
     (network, authorised) prints the installer's mismatch report and a line of its own
     saying that the failure is tolerated, still runs every line of
     `scripts/initialize.sh` after that one, and exits 0, with the install byte for byte
     as it was; then the pin is restored (`cmp` against the saved copy) and
     `sh scripts/install-quint-studio.sh --check` exits 0.
6. **Nothing from `PATH`.** Show that the installer calls no external utility but
   `curl`, `shasum`, `sed`, `mktemp`, `chmod`, `stat`, `mv`, `rm`, `uname` and `sh` (for
   the pin check), shell builtins aside, and that a `quint-studio` on `PATH`, from a
   stub under `ai_tmp/qs03/`, changes nothing it does. Show the mode check reading the
   exact mode: `chmod 0500` on the installed AppImage, which still passes `test -x` and
   fails `test -w`, makes `--check` exit 1; restored, `--check` exits 0.
7. **No network.** Over the stamped install, run `just install-quint-studio` under
   §11's macOS denial, `sandbox-exec -p '(version 1)(allow default)(deny network*)'`,
   with both of §11's `curl` negative controls failing inside the same denial, and quote
   all three exit codes (claim 3). The installer's run must exit 0 and print its own
   skip message, the one step 5's second run printed, and the quote shows it. Exit 0
   alone is not the proof: an installer that tried the network, failed and went on would
   exit 0 too.
8. **Rebase and words.** Rebase onto, or merge, a base that holds QS02. If the step 4
   ruling put the installer in `just initialize`, put its line in
   `scripts/initialize.sh` where Files touched says, now that the base shows whether
   QS02's line is there. Remove `.tools/quint-studio/`, so that step 5's first run
   installs afresh, and rerun every quoted command of steps 4 to 7 there (step 5's runs,
   its `just initialize` runs among them if the ruling made them, and step 7's reach the
   network, authorised), replacing each earlier quote, since the rebase made them stale
   (§11, "Quoted evidence names its commit"). Edit each page in Files touched. Run
   `just check-docs` and `just check-agents`.
9. **Adversarial review** (checkpoint 2: conformance and breakage, over the diff with the
   pages included, before the final commit). A fix made here, like the baseline, every
   quote and any later request to push, follows `.scratch/quint-studio/README.md`,
   "Rules every ticket holds". Questions:
   - Does `lock-check` stay offline and read-only, and does `just check` stay free of the
     network?
   - Could Verification pass while the installed AppImage's hash differed from the pin?
   - Does every page agree on what the pin guarantees and what it does not?
   - If the installer is in `just initialize`, can a stale pin fail a new worktree's
     first run, or hide the failure of another line?
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

- `tools/quint-studio/studio.txt` names the URL, the version string and the hash the
  maintainer reviewed, and the review date; each mutation of step 4 makes
  `just lock-check` fail, each quoted.
- `just install-quint-studio` installs on a match, skips a stamped install, replaces a
  changed or re-moded one, the one whose hash alone changed at mode `555` among them,
  refuses a mismatch with both hashes and the re-pin procedure while leaving any
  existing install unchanged and no temporary directory, refuses a symlinked `.tools` or
  `.tools/quint-studio`, and a symlink at the AppImage's path or the stamp's whose
  target is otherwise valid, leaving its target unchanged, and refuses another host,
  each quoted.
- `--check` exits 1 on a mode other than `0555`, `0500` among them, quoted.
- `just install-quint-studio` over a stamped install exits 0 and prints its own skip
  message under the network denial, with both negative controls failing.
- If the step 4 ruling puts the installer in `just initialize`: `scripts/initialize.sh`
  runs it, `just initialize` over a stamped install exits 0, and over a stale pin it
  reports the installer's failure, runs the lines after it and exits 0, each quoted, at
  step 5 and again at step 8.
- `AGENTS.md` lists `just install-quint-studio` among the recipes that reach the
  network, and `just check-agents` passes.
- The pages say what the pin guarantees and what it does not: it detects drift, the
  installer cannot rely on fetching an older build once `stable/latest` has moved,
  nothing checks the version against the build, and it does not cover what Studio does
  at run time (ruling 9), which decision 0022 and `docs/explanation/security-model.md`
  say as QS01's Open point 7 lists it.
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
pin_file=tools/quint-studio/studio.txt
appimage=.tools/quint-studio/quint-studio.AppImage
out=ai_tmp/qs03/verify
mkdir -p "$out"
cp "$pin_file" "$out/studio.txt"
restore() {
    cp "$out/studio.txt" "$pin_file"
    if [ -f "$appimage" ]; then chmod 0555 "$appimage"; fi
}
trap restore EXIT

# Green first: the offline check passes, so the install below cannot download.
just lock-check
sh scripts/install-quint-studio.sh --check
just install-quint-studio >| "$out/install.log" 2>&1
pin=$(sed -n 's/^sha256=//p' "$pin_file")
test -n "$pin"
grep -Fx "sha256=$pin" .tools/quint-studio/.pachinko-stamp
test -f "$appimage"
sum=$(shasum -a 256 "$appimage")
test "${sum%% *}" = "$pin"
mode=$(stat -f '%Lp' "$appimage")
test "$mode" = 555

# Red: a malformed reviewed= fails the gate with the pin check's own message.
sed 's/^reviewed=.*/reviewed=2026-13-01/' "$out/studio.txt" >| "$pin_file"
rc=0; just lock-check >| "$out/lock-check.log" 2>&1 || rc=$?
cp "$out/studio.txt" "$pin_file"
test "$rc" -eq 1
grep -F 'tools/quint-studio/studio.txt: reviewed:' "$out/lock-check.log"
git diff --exit-code "$pin_file"

# Red: a mode that test -x would pass is refused by the exact mode check.
chmod 0500 "$appimage"
rc=0; sh scripts/install-quint-studio.sh --check >| "$out/check.log" 2>&1 || rc=$?
chmod 0555 "$appimage"
test "$rc" -eq 1
grep -F 'just install-quint-studio' "$out/check.log"
sh scripts/install-quint-studio.sh --check

just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

The last `--check` is the green run after the red one: a check that refused a correct
install would fail it. The EXIT trap restores the pin file and the mode if any line
fails between a mutation and its restore. Steps 4 to 7 hold the other mutations, the
refusal on a mismatch, the network denial with the installer's skip message, and the
`just initialize` runs if the ruling made them, whose output is quoted there.

## Open points

- Whether `just initialize` runs `just install-quint-studio`, tolerating its failure on
  a stale pin. The design proposes it does not; the maintainer rules at step 4.

## Hand-back notes

Empty until the ticket runs. Sections, in order: Base and branch; Baseline; QS01's facts
and the amendments; Ruling; Claims; What landed; The gate and the installer, red and
green; Checkpoint 1 record; Checkpoint 2 record; Departures; Verification; Open points.
