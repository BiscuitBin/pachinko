---
id: AP02
title: "Apalache pinned by checksum, installed by just initialize"
status: todo
depends_on: [AP01]
parallel_with: []
branch: ticket/ap02-checksummed-apalache
estimated_size: S
---

# AP02: Apalache pinned by checksum, installed by `just initialize`

## Context

After AP01, `just quint verify` works. It runs Apalache 0.62.4 on the `openjdk` pinned
in `pyproject.toml`, but Quint still downloads Apalache itself, into
`.quint/apalache-dist-0.62.4/`, with no checksum. Decision 0021,
`docs/decisions/0021-project-managed-quint-cli.md`, records that gap under
"Consequences": "the downloads happen inside the CLI, which offers no hook for a
checksum".

The CLI does offer a way around it. Read from the installed package under
`tools/quint/node_modules/@informalsystems/quint/`:

- `dist/src/apalache.js` line 128: `DEFAULT_APALACHE_VERSION_TAG = '0.62.4'`, the
  version `verify` uses unless `--apalache-version` says otherwise.
- `fetchApalache` (from line 334) returns
  `<QUINT_HOME>/apalache-dist-<version>/apalache/bin/apalache-mc` without downloading
  when that file exists, and logs `Using existing Apalache distribution in <path>` at
  debug verbosity. Otherwise it prints `Downloading Apalache distribution <version>...`
  and fetches
  `https://github.com/apalache-mc/apalache/releases/download/v<version>/apalache.tgz`.
- `config.js` line 37, `apalacheDistDir`, builds `<QUINT_HOME>/apalache-dist-<version>`,
  and the `Justfile` sets `QUINT_HOME` to `.quint`.

So a recipe that downloads that archive, checks its SHA-256 against a value this
repository holds, and unpacks it to the path Quint looks for, replaces Quint's unchecked
download. That is the pattern decision 0005,
`docs/decisions/0005-project-managed-allium-cli.md`, set for the Allium checker: a
version and a checksum per artefact, an installer into a gitignored directory, and
`just install-allium` in `just initialize`. Allium's pins live in the
`biscuit-games-tooling` package. Apalache's live in this repository.

Three things make the naive version wrong, and the design below answers each:

- **The installed package is not in CI.** `DEFAULT_APALACHE_VERSION_TAG` lives only in
  `tools/quint/node_modules/`, which `just install-quint` fills. The CI gate jobs run
  `just sync`, not `just install-quint` (decision 0021). A gate that reads the tag would
  find nothing to read in CI. A gate that skips when the package is absent could never
  fire there.
- **A skip-if-present installer would bless an unchecked copy.** AP01's run left Quint's
  own download in `.quint/apalache-dist-0.62.4/`. An installer that skips whenever
  `apalache-mc` exists would treat that copy as verified. So would a stamp kept beside
  `apalache/`: remove the tree, let Quint download it again, and the old stamp still
  matches.
- **Apalache's version follows Quint's.** Quint pins it, so a separate Renovate update
  for Apalache would always disagree with the CLI. The pin moves with a Quint bump, by
  hand, with its checksum.

Read these:

- `docs/decisions/0005-project-managed-allium-cli.md` and
  `docs/decisions/0021-project-managed-quint-cli.md`.
- `Justfile`: `install-allium`, `install-quint`, `lock-check` and the `QUINT_HOME`
  comment. `scripts/initialize.sh`.
- `renovate.json`, the rule whose `groupName` is `quint`.
- `docs/how-to/maintain-dependencies.md`, "The Quint CLI".
- `.scratch/apalache/AP01-java-and-verify.md`, "Adversarial review", for the review
  deltas, and its hand-back notes for what AP01 found about the launcher and port 8822.
- `tickets/CONVENTIONS.md` §11, §13 and §14.

## Goal

`just initialize` installs Apalache into `.quint/apalache-dist-<version>/` from an archive
whose SHA-256 this repository records. While that install is in place, `just quint
verify` uses it and downloads nothing. The installer refuses a pin that disagrees with
the installed CLI's Apalache version. Gate 2, `just lock-check`, fails offline, in CI as
locally, on any move of the Quint CLI's pin until the pin file records that its
Apalache pin was reviewed against the new CLI.

What stays open, and the pages say so: Quint downloads Apalache unchecked when `.quint/`
holds no install (a deleted directory, or a worktree where `just install-apalache` never
ran), and when a run passes `--apalache-version`. Gate 2 checks that the review was
recorded, not that it was right. The installer and Open points cover both gaps.

## Design proposed in this ticket

These are proposals. The maintainer confirms them, or changes them, after checkpoint 1
and before step 4, and the hand-back records the ruling. After that the builder does
not reopen them. Open points holds the alternatives. The design differs from a first
sketch in which gate 2 compared the pin with the CLI's `DEFAULT_APALACHE_VERSION_TAG`.
That tag lives only in the installed package, which CI does not have, so gate 2 checks
that the pin was reviewed, and the installer checks the tag.

- **One pin file, `tools/quint/apalache.txt`.** Comment lines start with `#`. Three
  `key=value` lines follow: `quint=` the Quint CLI version the pin was reviewed against,
  `apalache=` the Apalache version that CLI's `DEFAULT_APALACHE_VERSION_TAG` names, and
  `sha256=` the archive's hash, lowercase hex. The scripts read it with `sed` and never
  source it.
- **`scripts/check-apalache-pin.sh`, in `lock-check`.** Offline and read-only. It fails
  unless the file has exactly the three keys, `apalache` is `x.y.z`, `sha256` is 64
  lowercase hex digits, and `quint` equals both the `@informalsystems/quint` version in
  `tools/quint/package.json` and the version `tools/quint/package-lock.json` locks. It
  reads only committed files, so it fires in CI without the installed package. Every
  Quint bump therefore fails gate 2 until someone edits `quint=`, which the bump how-to
  says to do only after reading the new CLI's tag and moving `apalache=` and `sha256=`
  if it moved.
- **The stamp lives inside the tree it certifies.** The installer writes
  `.quint/apalache-dist-<v>/apalache/.pachinko-stamp` with two lines: `archive=` the
  pinned hash, and `tree=` a digest of the unpacked tree. The digest is the SHA-256 of
  the sorted list of `<sha256>  <path>` lines for every file under `apalache/` except
  the stamp. Removing `apalache/` removes the stamp with it, and a tree Quint unpacks
  has none.
- **`scripts/install-apalache.sh`, run by `just install-apalache`.** In order:
  1. It reads the pin file and fails if `scripts/check-apalache-pin.sh` would.
  2. It fails, naming `just install-quint`, unless the installed package's `version` is
     the `quint` pin. It then fails, naming both versions, unless the package's
     `DEFAULT_APALACHE_VERSION_TAG` is the `apalache` pin. This is the one place the tag
     itself is checked.
  3. It skips, and says so, only if the stamp exists, its `archive=` is the pinned hash,
     its `tree=` equals the digest recomputed now, and `apalache/bin/apalache-mc` is
     executable. The digest covers content, not permissions, so the last test is
     separate. Anything else under
     `.quint/apalache-dist-<v>/apalache/`, Quint's own download included, is removed
     and reinstalled.
  4. It downloads the archive with `curl -fsSL --proto '=https'` into a new temporary
     directory `.quint/install-apalache.XXXXXX` (from `mktemp -d`), checks its SHA-256
     (`sha256sum`, or `shasum -a 256` where that is missing), and on a mismatch prints
     both hashes, removes the temporary directory and exits non-zero.
  5. It unpacks into the temporary directory, checks that `apalache/bin/apalache-mc`
     exists there, writes the stamp into the unpacked tree, and only then replaces
     `.quint/apalache-dist-<v>/apalache` with it. An interrupted run leaves the old tree
     or none, never an unstamped tree that looks finished.
  6. Every `rm -rf` names its path through `"${var:?}"`, and a trap removes the
     temporary directory on every exit.
- **`just initialize` runs `just install-apalache` after `just install-quint`.**
- **Renovate does not track Apalache.** No manager reads `tools/quint/apalache.txt`. The
  `quint` rule's description says that its pull request fails `lock-check` until the pin
  file is reviewed, and how to review it.

## Claims this ticket checks

| # | Claim | Check |
| --- | --- | --- |
| 1 | The v0.62.4 `apalache.tgz` is one archive for every platform, so one hash serves `linux-64` and `osx-arm64` | the release's asset list, step 2 |
| 2 | Apalache's release publishes a checksum of its own, which the computed hash can be compared with | the release's asset list and notes, step 2; if none, say so, and the hash rests on one download over TLS |
| 3 | With a stamped install in place, `just quint verify --verbosity=4` logs `Using existing Apalache distribution` and never `Downloading Apalache distribution` | step 6. Read from `dist/src/verbosity.js`, not run: the default level is 2 and debug lines print at 4 or above |
| 4 | `just install-apalache` over a stamped install reaches no network | step 6, under §11's denial with its negative controls |

## Non-goals

- No checksum for the Rust evaluator Quint downloads for `run`, `test` and the REPL. It
  has the same gap and could follow this pattern in a later ticket. Decision 0021 keeps
  saying so.
- No gate or CI job that runs `quint verify`, and no `just install-apalache` in CI. There
  is no `.qnt` module yet.
- No guard in `just quint` that refuses `verify` without a stamped install or with
  `--apalache-version`. Open points holds it, recommended as the next ticket.
- No change to how Quint picks its server endpoint.
- No custom Renovate manager for Apalache.
- No `CHANGELOG.md` entry. It records the crates' changes.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14, with the deltas
`.scratch/apalache/AP01-java-and-verify.md` writes under "Adversarial review". `<id>` is
`AP02`. There are two checkpoints, at steps 3 and 8.

## Files touched

Each page row covers every sentence on that page that says Apalache is downloaded on
first use, is checked by nothing, or lists what `just initialize`, gate 2 or the network
recipes do. Each such sentence is brought into line, keeping the gaps under Goal stated.

| Path | Class | Content |
| --- | --- | --- |
| `tools/quint/apalache.txt` | new | the pin file, with a comment naming decision 0021 and the bump how-to |
| `scripts/install-apalache.sh` | new | the installer, POSIX `sh` |
| `scripts/check-apalache-pin.sh` | new | the gate's check, POSIX `sh` |
| `Justfile` | shared, own lines | `install-apalache`, with a comment; a line in `lock-check`, and its comment; the `QUINT_HOME` and `quint` comments |
| `scripts/initialize.sh` | shared, own lines | `just install-apalache` after `just install-quint`, with a comment; the comments that count the first-run downloads no lockfile accounts for, and the one on the Quint CLI's downloads |
| `renovate.json` | shared, own lines | the `quint` rule's description |
| `.gitignore` | shared, own lines | the comment over `/.quint/` |
| `AGENTS.md` | shared, own lines | `just install-apalache` in the list of recipes that reach the network; the sentence on what Quint downloads on first use |
| `docs/decisions/0021-project-managed-quint-cli.md` | shared, own lines | Decision and Consequences: Apalache is pinned by checksum, with the gaps under Goal; the evaluator's gap stays; What would reopen this |
| `docs/how-to/develop-locally.md` | shared, own lines | what `just initialize` installs, and "Quint" |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | "The Quint CLI": the existing bump procedure and its sentence on unchecked downloads, and a subsection on moving the Apalache pin, reading the new tag and computing the hash; the description of gate 2 |
| `docs/operations/maintenance.md` | shared, own lines | the sentence on moving the Quint CLI's pin |
| `docs/reference/commands.md` | shared, own lines | an `install-apalache` row, the network list, and the `initialize`, `lock-check` and `quint` rows |
| `docs/reference/quality-gates.md` | shared, own lines | gate 2's row, what a manifest change needs before gate 2 passes, and the recipes that reach the network |
| `docs/reference/configuration.md` | shared, own lines | the `QUINT_HOME` row, a row for `tools/quint/apalache.txt`, and the pins table |
| `docs/reference/testing.md` | shared, own lines | only if its sentence on `lock-check` lists what gate 2 checks |
| `docs/explanation/security-model.md` | shared, own lines | the Quint CLI exception: Apalache is checked when installed by the recipe, with the gaps under Goal; the evaluator is not |
| `docs/project/repository-map.md` | shared, own lines | the `tools/quint/` line names the pin file; the `scripts/` line names the two scripts |
| `docs/tutorials/first-change.md` | shared, own lines | only if its list of what `just initialize` installs needs Apalache |
| `.scratch/apalache/AP02-checksummed-apalache.md` | ticket | `status:`, the hand-back notes, and a correction to Verification that a control shows is needed, recorded under Departures |
| `.scratch/apalache/README.md` | shared, own lines | AP02's Status cell |

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/ap02-checksummed-apalache`
   from a base that holds AP01, and record it and its full commit id as `base`. Run
   `just initialize` (network, authorised). Run `just check` on the untouched worktree
   and quote its last line, headed by the commit. A new worktree has no `.quint/`:
   AP01's download does not carry over.
2. **Read before writing** (network, authorised). Read the v0.62.4 release's asset list
   and notes (claims 1 and 2). Download `apalache.tgz` into a new, empty
   `ai_tmp/ap02/download/` and compute its SHA-256 with both tools the installer may
   use. If the release publishes a checksum, compare the two. Never copy a hash from
   this ticket, a page or a chat: there is none here to copy. Unpack a copy into a new,
   empty `ai_tmp/ap02/fixture/`; it stands for Quint's own download in step 5, and
   nothing in it is executed. Copy the scratch module from AP01's step 5 into
   `ai_tmp/ap02/smoke.qnt`. Write `ai_tmp/ap02/plan.md` with the pin file's content,
   each script's outline against "Design proposed", and the edit to each page in Files
   touched.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/ap02/plan.md`, before any tracked file changes). Questions:
   - Can `check-apalache-pin.sh` pass in CI while the Quint pin and `quint=` disagree?
     Does it read anything CI does not have?
   - Can the installer skip over a tree it did not unpack and check: Quint's download,
     a tree repopulated after `apalache/` was removed, or a tree with one file changed?
   - Can a Quint bump reach `main` with `apalache=` naming a version the new CLI does
     not use, and do the pages say so honestly?
   - Does each `rm -rf` fail closed when its variable is empty?
4. **Red: the gate.** **Stop** first for the maintainer's ruling on "Design proposed",
   with the checkpoint's findings in hand, and record it. Then add `tools/quint/apalache.txt` and `scripts/check-apalache-pin.sh`,
   and the `lock-check` line. Save the pin file to `ai_tmp/ap02/apalache.txt.saved`.
   Show `just lock-check` passing. Then, each on an edit restored from the saved copy
   afterwards, show it failing and quote each message: `quint=` changed to `0.33.0`;
   the `sha256=` line removed; `sha256=` given 63 digits; a fourth key added. After the
   last restore, `cmp tools/quint/apalache.txt ai_tmp/ap02/apalache.txt.saved` exits 0.
5. **The installer.** Add `scripts/install-apalache.sh` and `just install-apalache`, and
   the line in `scripts/initialize.sh`. Then, quoting each run's output and exit status,
   and restoring each edit afterwards:
   - copy `ai_tmp/ap02/fixture/apalache` to `.quint/apalache-dist-0.62.4/apalache`, as
     Quint would leave it. `just install-apalache` (network, authorised) replaces it and
     writes the stamp;
   - a second `just install-apalache` skips and says so;
   - append a line to one file under the installed `apalache/` that is not the stamp.
     The installer replaces the tree (network, authorised);
   - remove `.quint/apalache-dist-0.62.4/apalache`, copy the fixture back in, and the
     installer replaces it, because the stamp went with the removed tree (network,
     authorised);
   - `chmod a-x` the installed `apalache/bin/apalache-mc`. The installer replaces the
     tree (network, authorised);
   - with `apalache=0.62.3` (the pin file otherwise unchanged, so the gate's check
     passes), the installer fails and names both versions;
   - with the installed package's `package.json` `version` edited to `0.33.0` (save the
     file under `ai_tmp/ap02/` first, restore it, and show `cmp` of the two exits 0),
     the installer fails and names `just install-quint`;
   - with `sha256=` changed by one digit and `apalache/` removed, the installer (network,
     authorised) fails,
     prints both hashes, leaves no `apalache/` and no `.quint/install-apalache.*`;
   - restore the pin file and run `just install-apalache` once more (network,
     authorised), so a stamped install is in place for step 6.
6. **Quint uses the checked copy.** With AP01's port probe showing port 8822 free, run
   `just quint verify --verbosity=4 --invariant=inRange --max-steps=5
   ai_tmp/ap02/smoke.qnt`. Quote the `Using existing Apalache distribution` line, and
   show that `Downloading Apalache distribution` does not appear (claim 3). Then run
   `just install-apalache` under §11's macOS denial,
   `sandbox-exec -p '(version 1)(allow default)(deny network*)'`, with both of §11's
   `curl` negative controls failing inside the same denial, and quote all three exit
   codes (claim 4).
7. **Words.** Edit each page in Files touched. Run `just check-docs` and
   `just check-agents`.
8. **Adversarial review** (checkpoint 2: conformance and breakage, over the diff with
   the pages included, before the final commit). Questions:
   - Would gate 2 fail on a Renovate pull request that moves the Quint CLI and touches
     nothing else?
   - Could Verification pass while Quint downloaded Apalache itself?
   - Does `lock-check` stay offline and read-only, and does `just check` stay free of
     the network?
   - Does every page in Files touched agree on where the pin lives, what checks it, and
     the gaps under Goal? Does any page outside Files touched still say otherwise?
9. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed by
   the full commit id. Set `status: done` here and in `.scratch/apalache/README.md`,
   write the hand-back notes, and commit the record. **Stop** before pushing or opening a
   pull request: both need authorisation (`tickets/CONVENTIONS.md` §11, "Separately
   authorised actions").

## Acceptance criteria

- `tools/quint/apalache.txt` names `quint=0.34.0`, `apalache=0.62.4` and the hash step 2
  computed, and each mutation of step 4 makes `just lock-check` fail, each quoted.
- `just install-apalache` replaces an unstamped tree, a changed tree and a repopulated
  tree, a tree whose launcher is not executable, skips a stamped one, and refuses a mismatched tag, a mismatched CLI and a wrong
  hash, leaving no tree and no temporary directory after the last, each quoted.
- `just initialize` runs `just install-apalache` after `just install-quint`.
- `just quint verify --verbosity=4` logs `Using existing Apalache distribution` and no
  download line.
- `just install-apalache` over a stamped install exits 0 under the network denial, with
  both negative controls failing.
- `AGENTS.md` lists `just install-apalache` among the recipes that reach the network,
  and `just check-agents` passes.
- The pages state the gaps under Goal.
- Each of the four claims has its outcome recorded.
- Both checkpoints are recorded as §14's Record asks, with AP01's deltas.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run from a file as one `sh` script; `set -eu` stops it at the first command that fails.
Each probe that passes by printing nothing follows its positive control.

```sh
set -eu
git rev-parse HEAD

just lock-check
cp tools/quint/apalache.txt ai_tmp/ap02/apalache.txt.saved
sed 's/^quint=.*/quint=0.33.0/' ai_tmp/ap02/apalache.txt.saved >| tools/quint/apalache.txt
if just lock-check; then cp ai_tmp/ap02/apalache.txt.saved tools/quint/apalache.txt; exit 1; fi
cp ai_tmp/ap02/apalache.txt.saved tools/quint/apalache.txt
git diff --exit-code tools/quint/apalache.txt

just install-apalache
pin=$(sed -n 's/^sha256=//p' tools/quint/apalache.txt)
test -n "$pin"
grep -Fx "archive=$pin" .quint/apalache-dist-0.62.4/apalache/.pachinko-stamp
test -x .quint/apalache-dist-0.62.4/apalache/bin/apalache-mc

mkdir .quint/install-apalache.control
test -n "$(find .quint -maxdepth 1 -name 'install-apalache.*' -print)"
rmdir .quint/install-apalache.control
leftover=$(find .quint -maxdepth 1 -name 'install-apalache.*' -print)
test -z "$leftover"

just quint verify --verbosity=4 --invariant=inRange --max-steps=5 ai_tmp/ap02/smoke.qnt >| ai_tmp/ap02/verify.log 2>&1
grep -F 'Using existing Apalache distribution' ai_tmp/ap02/verify.log
if grep -F 'Downloading Apalache distribution' ai_tmp/ap02/verify.log; then exit 1; else test "$?" -eq 1; fi

just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

The script runs after the commit, so `git diff --exit-code` compares the committed pin
file. `grep -F 'Using existing…'` is the positive control for the silent probe after it:
both read the same log. Steps 4 to 6 hold the other mutations and the network denial,
whose output is quoted there.

## Hand-back notes

### 1. Base and branch

### 2. Baseline

### 3. Claims

### 4. What landed

### 5. The gate and the installer, red and green

### 6. Checkpoint 1 record: the plan

### 7. Checkpoint 2 record: the diff

### 8. Departures

### 9. Verification

## Open points

- **A guard in `just quint`, recommended next.** Without one, Quint downloads Apalache
  unchecked when `.quint/` holds no stamped install, and whenever a run passes
  `--apalache-version`. A guard would run before the CLI when any argument is `verify`,
  or `compile` with `--target tlaplus`. It would check the stamp offline, as the
  installer's skip path does, and refuse `--apalache-version`. It is left out because it
  parses the CLI's arguments in a recipe, and the maintainer should choose that.
- **Checking the tag in CI.** Gate 2 checks that the review was recorded, not that
  `apalache=` equals the CLI's tag. Adding `just install-quint` to the gate job would
  let CI check the tag too, at the cost of a network step in that job.
- **Where the pin lives.** The alternative to `tools/quint/apalache.txt` is a key in
  `tools/quint/package.json`, which npm ignores. It would keep one file per Quint bump,
  but Renovate's npm manager rewrites that file.
- **The evaluator.** The Rust evaluator for `run`, `test` and the REPL is still
  downloaded unchecked. This ticket's pattern would serve it, with one archive per
  platform.
