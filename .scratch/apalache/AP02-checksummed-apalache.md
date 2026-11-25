---
id: AP02
title: "Apalache pinned by checksum, installed by just initialize"
status: done
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
Apalache pin was reviewed against the new CLI. `just quint` refuses `verify`, and
`compile` with `--target tlaplus`, unless that stamped install is in place, and refuses
`--apalache-version` outright, so Quint never downloads Apalache itself through the
recipe.

What stays open, and the pages say so: gate 2 checks that the review was recorded, not
that it was right. Open points covers it.

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
- **A guard in `just quint`** (maintainer ruling, 2026-11-24, moved here from Non-goals
  and Open points). Before the CLI runs, the recipe scans every argument:
  1. It refuses `--apalache-version` and `--apalache-version=<v>`, naming the pin file.
  2. When any argument is `verify`, or one is `compile` and the target is `tlaplus`
     (`--target tlaplus` or `--target=tlaplus`), it runs
     `scripts/install-apalache.sh --check`. That mode is offline and read-only: it runs
     the installer's checks 1 and 2 and step 3's skip test, and exits 0 only when that
     test would skip. Otherwise it fails, naming `just install-apalache`. It also fails,
     naming `just sync`, unless pixi's `java` (`.pixi/envs/default/lib/jvm/bin/java`,
     which the recipe puts first on `PATH`) is executable: without it the launcher's
     bare `java` falls through to whatever else is on `PATH` (handed over by AP01's
     checkpoint 2).
  3. Every other command, `typecheck`, `run`, `test` and the REPL among them, runs as
     before, with no check.

  The scan matches any argument, not only the subcommand, so a file named `verify`
  triggers the check too. That fails closed, and the message says why.
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
| `scripts/install-apalache.sh` | new | the installer, POSIX `sh`, with a `--check` mode for the guard that reuses its offline skip test |
| `scripts/check-apalache-pin.sh` | new | the gate's check, POSIX `sh` |
| `Justfile` | shared, own lines | `install-apalache`, with a comment; a line in `lock-check`, and its comment; the guard in the `quint` recipe, with its comment; the `QUINT_HOME` and `quint` comments |
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

   Then add the guard to the `quint` recipe, and show, quoting each:
   - with the stamp moved to `ai_tmp/ap02/stamp.saved`, `just quint verify` and
     `just quint compile --target tlaplus` on `ai_tmp/ap02/smoke.qnt` each fail, naming
     `just install-apalache`, and `.quint/apalache-dist-0.62.4/` is unchanged (no
     download started), while `just quint typecheck` and
     `just quint run --max-steps=3` on the same module pass;
   - with one file under the installed `apalache/` changed, `just quint verify` fails
     the same way (the guard recomputes the digest); reinstall afterwards;
   - with the stamp restored (`cmp` exits 0), `just quint verify --apalache-version=0.62.4`
     fails, naming the pin file, and `--apalache-version 0.62.4` does too;
   - with `.pixi/envs/default/lib/jvm/bin/java` moved to `ai_tmp/ap02/java.saved` and
     back afterwards (`just sync` restores it if the move back fails), `just quint verify`
     fails, naming `just sync`, and `just quint typecheck` still passes.
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
- `just quint` refuses `verify` and `compile --target tlaplus` without a stamped install
  and with a changed tree, refuses `--apalache-version` in both spellings, and runs
  `typecheck` and `run` untouched, each quoted.
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

# The guard: refuse without the stamp, refuse --apalache-version, pass the rest.
stamp=.quint/apalache-dist-0.62.4/apalache/.pachinko-stamp
mv "$stamp" ai_tmp/ap02/stamp.saved
if just quint verify --invariant=inRange --max-steps=5 ai_tmp/ap02/smoke.qnt; then mv ai_tmp/ap02/stamp.saved "$stamp"; exit 1; fi
if just quint compile --target tlaplus ai_tmp/ap02/smoke.qnt; then mv ai_tmp/ap02/stamp.saved "$stamp"; exit 1; fi
just quint typecheck ai_tmp/ap02/smoke.qnt
just quint run --max-steps=3 ai_tmp/ap02/smoke.qnt
mv ai_tmp/ap02/stamp.saved "$stamp"
grep -Fx "archive=$pin" "$stamp"
if just quint verify --apalache-version=0.62.4 --invariant=inRange --max-steps=5 ai_tmp/ap02/smoke.qnt; then exit 1; fi
if just quint verify --apalache-version 0.62.4 --invariant=inRange --max-steps=5 ai_tmp/ap02/smoke.qnt; then exit 1; fi
just quint verify --invariant=inRange --max-steps=5 ai_tmp/ap02/smoke.qnt

just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
```

The guard's refusals are the red tests and the last `verify` the green one: a refusal
that also refused a stamped install would fail that line. The script runs after the
commit, so `git diff --exit-code` compares the committed pin file. `grep -F 'Using existing…'` is the positive control for the silent probe after it:
both read the same log. Steps 4 to 6 hold the other mutations and the network denial,
whose output is quoted there.

The script above did not run as written. Checkpoint 2 required a correction, so it was
replaced by `ai_tmp/ap02/verify.sh`, which stays in `ai_tmp/`, uncommitted. Departures,
item 12, says what it adds, and section 9 quotes its run.

## Hand-back notes

### 1. Base and branch

Base `quint-spec` at `4933c3a66de6d11075c2433ec0b0b305b250eb9e`, which is itself the
amendment that moved the maintainer's guard ruling into "Design proposed". The work ran
on `quint-spec` in this worktree, not on `ticket/ap02-checksummed-apalache` (Departures,
item 1). It landed as `ffe512b85e7ff9a29765d5130fae7fe4cbff5a76`.

### 2. Baseline

At `4933c3a`, `just check` on the untouched worktree, exit 0:

```text
All checks passed and the worktree is unchanged.
```

### 3. Claims

1. **Holds.** Read at `4933c3a` (step 2). The v0.62.4 release lists five assets:
   `apalache-0.62.4.tgz`, `apalache-0.62.4.zip`, `apalache.tgz`, `apalache.zip` and
   `sha256sum.txt`. None is per platform, and AP01 listed `apalache.tgz` as one top-level
   `apalache/` holding the jar and two launchers.
2. **Holds.** Read at `4933c3a` (step 2). The release publishes `sha256sum.txt`. Its
   `apalache.tgz` line, GitHub's asset digest
   (`gh api repos/apalache-mc/apalache/releases/tags/v0.62.4`, `.assets[].digest`), and the
   download in `ai_tmp/ap02/download/` hashed with `shasum -a 256`, `sha256sum`
   (`/sbin/sha256sum` on this macOS) and `openssl dgst -sha256` all give
   `8ac8488422c30d552735bae11b92a2d014d8e02e8608702acc08f9a2d8a9abb2`. That download is
   byte-identical (`cmp`) to AP01's download of the same URL. So the pin does not rest on
   one download alone.
3. **Holds**, at `ffe512b`, through step 6's driver (the quote is under claim 4).
4. **Holds**, in the same run:

   ```text
   ffe512b
   (uncommitted work on top)
   ## port 8822 control
   97856
   control lsof exit 0 (listener pid 97856)
   lsof exit 1
   ## verify --verbosity=4 on the stamped install
   verify exit 0
   Using existing Apalache distribution in .quint/apalache-dist-0.62.4/apalache/bin/apalache-mc
   grep using exit 0
   grep downloading exit 1 (1 = absent)
   [ok] No violation found (3163ms).
   ## install-apalache under network denial
   curl: (6) Could not resolve host: static.rust-lang.org
   control 1 curl exit 6
   curl: (7) Failed to connect to 1.1.1.1 port 443 after 0 ms: Couldn't connect to server
   control 2 curl exit 7
   sh scripts/install-apalache.sh
   install-apalache: Apalache 0.62.4 is installed and matches its stamp; nothing to do.
   install-apalache under denial exit 0
   ```

   The second line is fixed text the driver always echoes, not a reading of the tree.
   By file times, the run started after the commit and after `verify.sh` had found
   `git status --short` empty (section 9). The denial was
   `sandbox-exec -p '(version 1)(allow default)(deny network*)'`, with both `curl`
   controls inside it. The same run at `4933c3a` with the work uncommitted gave the same
   lines.

### 4. What landed

`ffe512b`, `git show --stat`:

```text
 .gitignore                                       |   5 +-
 AGENTS.md                                        |   2 +-
 Justfile                                         |  72 ++++-
 docs/decisions/0021-project-managed-quint-cli.md |  74 +++--
 docs/explanation/security-model.md               |  16 +-
 docs/how-to/develop-locally.md                   |  24 +-
 docs/how-to/maintain-dependencies.md             |  56 +++-
 docs/operations/maintenance.md                   |   3 +-
 docs/project/repository-map.md                   |   4 +-
 docs/reference/commands.md                       |  21 +-
 docs/reference/configuration.md                  |   6 +-
 docs/reference/quality-gates.md                  |   6 +-
 docs/reference/testing.md                        |   2 +-
 docs/tutorials/first-change.md                   |   4 +-
 renovate.json                                    |   2 +-
 scripts/check-apalache-pin.sh                    |  66 +++++
 scripts/initialize.sh                            |  18 +-
 scripts/install-apalache.sh                      | 360 +++++++++++++++++++++++
 tools/quint/apalache.txt                         |  10 +
 19 files changed, 663 insertions(+), 88 deletions(-)
```

- `tools/quint/apalache.txt`: `quint=0.34.0`, `apalache=0.62.4` and the `sha256=` above,
  under a comment naming decision 0021, the bump how-to and the gap the gate leaves.
- `scripts/check-apalache-pin.sh`: the last line of `lock-check`. It reads the pin file,
  `tools/quint/package.json` and `tools/quint/package-lock.json` as they are in the
  checkout, with `sed` and pixi's `python -I -B`, and never `node_modules/`.
- `scripts/install-apalache.sh`: the installer and its `--check` mode, POSIX `sh`. The
  tree digest is one pixi `python -I -B` walk that fails on any read error or any entry
  that is not a regular file or directory. `tar` runs with its environment knobs unset,
  member names are checked before extraction, and the extracted set must equal the
  listing.
- `Justfile`: `install-apalache`, the `lock-check` line, the comments named in Files
  touched, and the `quint` recipe, which now reads:

  ```text
  sh scripts/install-apalache.sh --check -- "$@"
  env -u APALACHE_JAR -u BASH_ENV -u CDPATH TMPDIR="${TMPDIR:-/tmp}" PATH={{ quote(justfile_directory() / ".pixi" / "envs" / "default" / "lib" / "jvm" / "bin") }}:"$PATH" tools/quint/node_modules/@informalsystems/quint/dist/src/cli.js "$@"
  ```

- `scripts/initialize.sh`: `just install-apalache` after `just install-quint`, with a
  comment counting it as the third download no lockfile accounts for.
- `renovate.json`, `.gitignore`, `AGENTS.md` and every page in Files touched, with the
  checkpoints' wording fixes (sections 6 and 7). `docs/reference/testing.md` and
  `docs/tutorials/first-change.md`, both conditional rows, were edited.

### 5. The gate and the installer, red and green

These ran at `4933c3a` with the work uncommitted. The code changed after them in three
ways: the member-name check moved ahead of extraction (Departures, item 9, rerun in
`step5-followup.log`); checkpoint 2 added `env -u CDPATH` and the switch from
`node_modules/.bin/quint` to the package's `dist/src/cli.js`, with a new refusal of a
symlinked `cli.js`; and checkpoint 2 added the recovery sentence to the symlink refusal
(`Remove the symlink itself (not its target), then run just install-apalache`).

At `ffe512b`, `verify.sh` reran a subset (section 9): the `quint=0.33.0` gate mutation,
the installer's skip and its symlinked-tree refusal, and the guard's refusals without
the stamp, with a changed tree, for both `--apalache-version` spellings, without
`java`, for four `QUINT_HOME` values and for a symlinked tree. The other three gate
mutations, installer cases 1 to 12 (only the skip ran again, under `sh`), and the rest of the guard cases were not
rerun after those fixes; their quotes below stand at `4933c3a` with the work
uncommitted only.

**Step 4, the gate.** `just lock-check` passed (`exit=0`), and so did the check under
`dash`. Each mutation, restored from `ai_tmp/ap02/apalache.txt.saved` afterwards
(`cmp … exit=0` after each and after the last):

| Mutation | `just lock-check` | Message |
| --- | --- | --- |
| `quint=0.33.0` | exit 1 | `tools/quint/apalache.txt says quint=0.33.0, but tools/quint/package.json pins 0.34.0 and tools/quint/package-lock.json locks 0.34.0. After a Quint bump, read the new CLI's DEFAULT_APALACHE_VERSION_TAG, move apalache= and sha256= if it moved, then set quint= …` |
| `sha256=` removed | exit 1 | `must have exactly one sha256= line; it has 0` |
| 63 digits | exit 1 | `sha256= in tools/quint/apalache.txt is not 64 lowercase hex digits: 8ac8…abb` |
| `evaluator=0.8.0` added | exit 1 | `has a line that is not quint=, apalache=, sha256=, a comment or blank: evaluator=0.8.0` |

Each message starts `check-apalache-pin:`.

**Step 5, the installer.** Each case's exit and its first diagnostic line, from
`just install-apalache` unless noted:

| Case | Exit | Output |
| --- | --- | --- |
| 1, AP01's unchecked tree (identical to the fixture) | 0 | `no stamp at …/.pachinko-stamp`, download, `installed Apalache 0.62.4 (sha256 8ac8…abb2)` |
| 2, again | 0 | `Apalache 0.62.4 is installed and matches its stamp; nothing to do.` |
| 3, a line appended to `LICENSE` | 0 | `tree digest mismatch`, reinstalled |
| 4, `apalache/` removed, fixture copied back | 0 | `no stamp at …`, reinstalled |
| 5, `chmod a-x` the launcher | 0 | `launcher not executable`, reinstalled |
| 6, stamp `chmod 000` (extra) | 0 | `stamp unreadable`, reinstalled |
| 7, `apalache=0.62.3` (`lock-check` exit 0) | 1 | `the installed Quint CLI 0.34.0 runs Apalache '0.62.4' (DEFAULT_APALACHE_VERSION_TAG), but tools/quint/apalache.txt pins apalache=0.62.3` |
| 8, installed `version` `0.33.0` | 1 | `the installed Quint CLI is 0.33.0, not quint=0.34.0 from tools/quint/apalache.txt; run just install-quint` |
| 9, `sha256=` one digit off, tree removed | 1 | `checksum mismatch …: tools/quint/apalache.txt pins sha256 9ac8…, the download has 8ac8…; nothing was installed` |
| 10, GNU tar with `TAR_OPTIONS` excluding the jar (extra) | 0 | installed with the 197019382-byte jar |
| 11, `dash` over the stamped install (extra) | 0 | `nothing to do.` |
| 12, `QUINT_HOME` `ai_tmp/ap02/qh-empty`, `./.quint`, `.quint/`, absolute (extra) | 1 each | `QUINT_HOME must be .quint, the value the Justfile exports` |

After case 9: `tree: absent` and `temporary directories left: none`. After cases 7 and 8,
`cmp` of the pin and of `package.json` against their saved copies exited 0. Case 10's
positive control showed that the same `tar`, with `TAR_OPTIONS` left set, lists no jar.
The run ended with a skip over the stamped install, and `cmp` of both files exited 0.

**Step 5, the guard** (`step5-guard.log`). With the stamp moved aside,
`just quint verify`, `compile --target tlaplus`, `--target=tlaplus`, `--target="tlaplus"`,
`--target json`, `verify --help`, `--help verify …` and `--verbosity=4 verify …` each
exited 1 with:

```text
install-apalache --check: refused: Apalache 0.62.4 is not installed as tools/quint/apalache.txt pins it (no stamp at .quint/apalache-dist-0.62.4/apalache/.pachinko-stamp); run just install-apalache. Checked because the first argument 'verify' is not one of parse, typecheck, run, test, repl or docs, so the command may run Apalache
```

(`'compile'`, `'--help'` or `'--verbosity=4'` where that came first.) `typecheck`,
`run --max-steps=3`, `run --backend=typescript --max-steps=3`, `--help` and `--version`
exited 0, `typecheck verify` reached Quint (`error: file verify does not exist`), and the
dist directory was unchanged across them all. The same refusal, with its own reason,
covered the tree moved aside, a changed `LICENSE`, the stamp `chmod 000` and an added
`chmod 000` subdirectory (`walk failed`). With the stamp restored, every option name
holding `apalache` was refused, `--apalache-version=0.62.4`, `--apalache-version 0.62.4`,
`--apalacheVersion`, `--APALACHE-VERSION`, `--no-apalache-version`,
`---apalache-version`, `--apalache--version`, `--apalacheConfig` and
`typecheck --apalache-version=…` among them, with:

```text
install-apalache --check: refused: the option '--apalache-version=0.62.4' could make Quint use or download another Apalache; the version is pinned in tools/quint/apalache.txt and installed by just install-apalache
```

With pixi's `java` moved aside, `verify` and `compile` exited 1 with `pixi's java,
.pixi/envs/default/lib/jvm/bin/java, is missing or not executable; the launcher would run
another java on PATH; run just sync`, and `typecheck` passed. With a directory in its
place, `verify` alone was run, and refused the same way. Seven `QUINT_HOME` values other than `.quint`, symlinked and `..` forms included, were refused,
`typecheck` too, and none was created. A symlink at `.quint` and at the dist directory
was refused by both modes. On the stamped install, `verify --verbosity=4` logged
`Using existing Apalache distribution`, no download line, and left the dist directory
unchanged; `verify`, `compile --target tlaplus`, `verify --backend=apalache` and
`verify --apalache-config=<file>` passed the guard with exit 0.

**Follow-up** (`step5-followup.log`, after the member-name fix). Listings with
`apalache/../evil` and `other/x` were refused before extraction, a forced reinstall went
through the new order, and `--check` with no arguments or with `parse`, `typecheck`,
`run`, `test`, `repl` or `docs` first exited 0 without the stamp, while `verify` was
refused.

Raw logs stay in `ai_tmp/`.

### 6. Checkpoint 1 record: the plan

**Harness and model.** Codex CLI v0.172.0, `codex exec -m gpt-6.2-sol -s read-only`, in
AP01's corrected form, over `ai_tmp/ap02/plan.md` at `4933c3a`. Lenses were conformance
and breakage. Triage and apply were done by fresh Claude subagents, one writer at a
time. Round 1 had valid high findings, so a second round ran, the last §14 allows.

**Round 1.** 17 findings, 12 distinct (C1 = B1, C2 = B2, C5 = B4, C6 = B3, C7 = B7;
C8 and B8 overlap but are listed apart). Eight were high (C1–C4, B1–B4). 16 valid, one valid in part (B8),
0 invalid.

- C1/B1 (high), valid: Quint's parser accepts `--apalacheVersion`, `--APALACHE-VERSION`,
  `--no-apalache-version` and more. Every option name holding `apalache` is now refused,
  except `--apalache-config`.
- C2/B2 (high), valid, dissolved: `--target="tlaplus"` slipped past the scan. Every
  `compile` now needs the check, so the target is never parsed.
- C3 (high), valid: `just QUINT_HOME=x quint` sent Quint to another cache. Settled in
  round 2.
- C4 (high), valid: a symlinked `.quint` or dist directory let the installer delete a
  shared tree. Both modes refuse a symlink or non-directory there.
- C5/B4 (high), valid, limitation recorded: a guarded run racing the installer can
  download. A lock was declined; use is stated as non-concurrent (Open points).
- C6/B3 (high), valid: a `find | sort | hash` pipeline hid a failed walk. The digest is
  a pixi-python walk that fails on any error.
- C7/B7, valid: the ticket's Verification accepted any non-zero exit. Corrected
  (Departures, item 12).
- C8, valid: the plan named the mode `--guard`. It is `--check`, and the "file named
  `verify`" diagnostic is gone under the first-argument rule.
- B8, valid in part: `typecheck verify` no longer trips the guard; `verify --help` stays
  refused, since token matching cannot allow it safely.
- C9, valid: pages outside the plan's edits named Apalache's download. Edits added; the
  README prose handed back.
- B5, valid: `tar -xzf -C` took `-C` as the archive. Explicit operands and a checked
  listing.
- B6, valid: `test -x` passes a directory, and the launcher's bash reads `BASH_ENV`.
  `[ -f ] && [ -x ]`, and `env -u BASH_ENV`.

**Round 2** (the last). 9 findings, 7 distinct (C1 = B1, C3 = B4). Three were high (C1,
C2, B1). 9 valid, 0 invalid.

- C1/B1 (high), valid: `QUINT_HOME=link/../cache` resolved differently in the shell and
  in Node's `path.join`. Only `QUINT_HOME` exactly `.quint` is accepted.
- C2 (high), valid: `-L x/` misses a symlink, and ancestors were never tested. The same
  rule: `.quint` is one component, tested with `-L` and `-d`.
- C3/B4, valid: the guard driver passed on any non-zero exit. It now requires exit 1,
  the diagnostic, an unchanged dist snapshot and no start of Quint's node.
- C4, valid: the network sentences omitted the race. Each carries the non-concurrent
  qualification.
- C5, valid: the plan framed departures as hand-back items. They went to the maintainer
  before step 4.
- B2, valid: `TAR_OPTIONS` could drop the jar from listing and extraction alike. `tar`
  runs with `TAR_OPTIONS`, `TAR_READER_OPTIONS`, `TAR_WRITER_OPTIONS` and `TAPE` unset,
  and the extracted set must equal the listing.
- B3, valid: an unreadable stamp stopped install mode. It now reinstalls, and check mode
  refuses.

**Applied.** All of the above went into `plan.md` before any tracked file changed.

**Maintainer rulings.** Before the work (2026-11-24): the design accepted, with the
guard. After checkpoint 1, three departures accepted: the first-argument allow-list (no
arguments, `parse`, `typecheck`, `run`, `test`, `repl`, `docs`, or a sole `--help` or
`--version` need no check; everything else, `compile` to any target and `verify --help`
included, needs the stamped install and pixi's `java`); the refusal of every option
naming `apalache` except `--apalache-config`; and `QUINT_HOME` only `.quint`, as a real
directory.

**Handed back.** `.scratch/apalache/README.md:49-51` (C9), below.

**Re-run.** The checkpoint's verification is steps 4 to 6 (sections 3 and 5), at
`4933c3a` with the work uncommitted.

### 7. Checkpoint 2 record: the diff

**Harness and model.** The same as checkpoint 1, over `git diff 4933c3a` plus the
uncommitted changes. Round 1 had a valid high finding, so a second round ran over the
fix delta and `verify.sh`.

**Round 1.** 15 findings, 12 distinct (B3 = C1, B5 = C6, B6 = C3 and C4, B7 overlaps C5
and C7). One was high (B1). 15 valid, 0 invalid.

- B1 (high), valid: with `CDPATH` naming a decoy, the launcher's `cd` resolved into the
  decoy's `.quint/`. The recipe unsets `CDPATH`, and `verify.sh` runs a live `verify`
  with `CDPATH` set.
- C1/B3, valid: the ticket's Verification checked the last `verify` by exit status
  alone. Replaced by `verify.sh` (Departures, item 12).
- C2, valid, docs: `commands.md` promised replacement of a symlinked tree the ruling
  refuses. The refusal stands; the message and the page name the recovery.
- C3/B6, valid: gate 2 was said to write nothing; npm writes its cache log. Reworded.
- C4/B6, valid: "committed files" overstated what the check reads. "The three files as
  they are in the checkout", and "records that the pin was reviewed, not that the
  review was right".
- C5, valid: `.quint/` named as the value. `.quint` is the value, `.quint/` the
  directory, and the refusal has its own sentence.
- C6/B5, valid: the bump how-to read the tag before installing the new CLI. The order
  is now explicit.
- C7, valid: the README prose predates the guard. Handed back.
- B2, valid: the recipe ran `node_modules/.bin/quint`, which could name another CLI. It
  runs `dist/src/cli.js`, and the guard requires that to be a regular file.
- B4, valid: a `verify` that reports a violation fetches the evaluator, and a remote
  `--server-endpoint` connects out. Named in `AGENTS.md`, `commands.md` and decision
  0021.
- B7, valid: decision 0021's Context stated Quint's own download in the present tense.
  Marked as the CLI's behaviour, pointing at the Decision.
- B8, valid, pre-existing: `prek run --all-files` can provision a missing hook
  environment. Handed back.

**Round 2** (the last). 13 findings, 11 distinct (2B1 = 2C1, 2B2 = 2C4). None was high.
13 valid, 0 invalid. All were fixes to `verify.sh` or wording.

- 2C1/2B1, valid: `verify.sh` ran `just install-apalache` first, which downloads when
  the stamp fails. The read-only `--check -- verify` runs first and stops the run. The
  network denial half stays open.
- 2C2, valid: a signal between a move and its flag skipped restoration. Flags are armed
  first, with per-run backups.
- 2C3, valid: `die` left the watchdog armed. The EXIT trap stops it, and kills a pid
  only while it runs `apalache.jar`.
- 2B3, valid: the watchdog's `sleep 300` held the caller's pipe. It polls and holds no
  descriptor.
- 2C4/2B2, valid: Node splits `NODE_OPTIONS` at spaces. The sentinel path is relative.
- 2C5, valid: override probes matched only the prefix, and no probe covered a symlinked
  `cli.js`. Both added.
- 2B4, valid: `tree-aside` sat outside the snapshot. Snapshotted and compared.
- 2B5, valid: the shim precondition rejected a working checkout. The shim's entry is
  recorded and restored as found.
- 2C6, valid: decision 0021 named `QUINT_HOME` as `.quint/`. Reworded.
- 2C7, valid: the `Justfile` and installer header promised replacement of refused
  paths. Both say a symlink or non-directory is refused.
- 2C8, valid: npm writes its log wherever its configuration puts it. Reworded in
  `quality-gates.md` and `maintain-dependencies.md`.

**Applied** to the `Justfile`, `scripts/install-apalache.sh`, the pages and `verify.sh`,
before `ffe512b`.

**Re-run.** Section 9, at `ffe512b`. Section 3's step 6 run, at the same commit, is the
network denial.

**Left open.** The denial inside Verification (2C1's second half) and a `just` that
ignored TERM, which was neither observed nor probed. Both are in Open points.

**Handed back to the maintainer (§11).** These sit outside the lines this ticket may
edit:

- `.scratch/apalache/README.md:3-7` and `:44-51` (C7, B7, and checkpoint 1's C9). The
  introduction says both commands fail for want of Java, and Dependencies says Quint
  still downloads Apalache unchecked when `.quint/` is missing or a run passes
  `--apalache-version`, and that AP02 records the guard as future work.
- `.dockerignore:64-66` still says Quint downloads the evaluator and Apalache into
  `QUINT_HOME`.
- B8: `prek run --all-files` inside `just check` can provision a missing hook
  environment. It predates AP02 and wants a ticket of its own if `check` must be offline
  unconditionally.

### 8. Departures

1. **Branch and worktree.** The work ran on `quint-spec`, in this worktree, with no
   ticket branch, no new worktree and no rerun of `just initialize`, as the maintainer
   ruled on 2026-11-24.
2. **Reviewer model.** `gpt-6.2-sol`, not `gpt-6-astra`, by the user's choice. The
   harness is still Codex.
3. **Step 1's `.quint/`.** "A new worktree has no `.quint/`" was false here: the
   worktree kept AP01's unchecked download, so it was the first tree the installer
   replaced (section 5, case 1).
4. **The guard reads the first argument.** Under the ruling after checkpoint 1, the
   allow-list on the first argument replaces the Design's scan of every argument, so
   `typecheck verify` runs and a file named `verify` no longer triggers the check. Every
   `compile`, `verify --help`, `--help` or `--version` followed by anything, any leading
   option and an unknown first word are refused without the stamped install.
5. **Option names.** Every option whose name holds `apalache`, in any case, is refused,
   not only `--apalache-version` in two spellings. `--apalache-config` and
   `--apalache-config=<file>` are exempt; `--apalacheConfig` is refused.
6. **`QUINT_HOME`.** Both modes refuse any value but `.quint`, for every command, so
   `just QUINT_HOME=x …` stops working. `.quint`, the dist directory and the tree must
   be real directories.
7. **The guard is the installer's `--check` mode,** run as the recipe's first line, not
   a scan written in recipe lines. Both recipes call `sh scripts/<name>.sh`, as the
   `Justfile`'s other script recipes do.
8. **The `quint` recipe** runs the package's `dist/src/cli.js`, the package the guard
   reads, not `node_modules/.bin/quint`, and unsets `CDPATH` and `BASH_ENV` beside
   `APALACHE_JAR` (checkpoint 1 B6, checkpoint 2 B1 and B2).
9. **The installer's tree handling.** A symlink or non-directory at `.quint`, the dist
   directory or the tree is refused, naming the recovery, not "removed and reinstalled"
   as the Design's step 3 says. `tar` runs with `TAR_OPTIONS`, `TAR_READER_OPTIONS`,
   `TAR_WRITER_OPTIONS` and `TAPE` unset; member names are checked before extraction,
   and the extracted set against the listing after it. The digest is a pixi-python walk,
   not `sha256sum` or `shasum`, and hashes the sorted `<hex>  <path>` lines each ending
   in a newline. An unreadable stamp means a reinstall, not a stop.
10. **Step 5's guard cases.** The changed-tree case restored the file from a copy rather
    than reinstalling; the installer's own changed-tree case reinstalled. The allow-list
    words and the no-argument case ran through `--check` directly, so no REPL started.
    The extra cases in section 5 (an unreadable stamp, `TAR_OPTIONS`, `dash`, symlinks
    and `QUINT_HOME`) are beyond the ticket's list.
11. **`scripts/initialize.sh` comments.** "One of the two network downloads … no
    lockfile accounts for" became "one of the network downloads", and Apalache is "the
    third"; `install-hooks` is still not counted.
12. **Verification corrected.** Checkpoint 1's C7/B7 and checkpoint 2's C1/B3 required
    it. The ticket's script is replaced by `ai_tmp/ap02/verify.sh`, which adds:
    - the guard's read-only `--check -- verify` before `just install-apalache`, which
      must then print its skip line, so the run stops before anything could download;
    - every refusal required to exit exactly 1 with its own diagnostic, with no download
      line, no start of Quint's node (a `NODE_OPTIONS=--require` sentinel, after its
      positive control) and an unchanged snapshot of the dist directory;
    - refusals for a changed tree, `compile` without the stamp, both `--apalache-version`
      spellings, a missing `java`, four `QUINT_HOME` values, a symlinked tree for both
      modes, and a symlinked `cli.js`; and a `.bin/quint` shim redirected to a sentinel
      that never runs;
    - every `verify` run live through AP01's checks: the port and `pgrep` probes after
      their positive controls, Quint's own spawn, the server's executable resolved to
      pixi's `lib/jvm/bin/java` and its jar to this worktree's, the
      `APALACHE version: 0.62.4` banner, and no download line;
    - a live `verify` with `CDPATH` naming a decoy checkout, after a control showing that
      `CDPATH` sends bash's `cd` there;
    - the `belowTwo` violation run;
    - one EXIT trap that restores every mutation from per-run backups, and a watchdog on
      every run.

    The network denial stays in step 6 (Open points).

### 9. Verification

`verify.sh` ran at `ffe512b` and exited 0; under `set -eu` it reaches its last line only
when every command before it succeeded. Excerpts:

```text
ffe512b85e7ff9a29765d5130fae7fe4cbff5a76
openjdk version "21.0.13.1" 2026-11-17 LTS
lock-check with quint=0.33.0: exit 1
install-apalache: Apalache 0.62.4 is installed and matches its stamp; nothing to do.
passes typecheck: exit 0
refused nostamp-verify: exit 1
refused nostamp-compile: exit 1
passes nostamp-typecheck: exit 0
passes nostamp-run: exit 0
refused changed-verify: exit 1
refused changed-compile: exit 1
refused version-eq: exit 1
refused version-sp: exit 1
refused nojava-verify: exit 1
passes nojava-typecheck: exit 0
refused qh-ai_tmp_ap02_qh_empty: exit 1
refused qh-_quint_: exit 1
refused qh-___quint: exit 1
refused qh-_Users_scutting__supacode_repos_pachinko_quint_spec__quint: exit 1
refused link-verify: exit 1
refused link-install: exit 1
refused cli-link: exit 1
passes shim-typecheck: exit 0
lsof -iTCP:8822 -sTCP:LISTEN exit 1
pgrep -fl apalache exit 1
control: CDPATH sends the launcher's cd to …/ai_tmp/ap02/verify-out/decoy/.quint/apalache-dist-0.62.4/apalache/bin
ok: server pid 93912; executable …/.pixi/envs/default/lib/jvm/bin/java -> …/.pixi/envs/default/lib/jvm/bin/java; jar …/.quint/apalache-dist-0.62.4/apalache/bin/../lib/apalache.jar -> …/.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar
ok exit 0
Using existing Apalache distribution in .quint/apalache-dist-0.62.4/apalache/bin/apalache-mc
# APALACHE version: 0.62.4 | build: 7954daf
[ok] No violation found (3125ms).
ok-cdpath: server pid 94410; executable …/.pixi/envs/default/lib/jvm/bin/java -> …/.pixi/envs/default/lib/jvm/bin/java; jar …/.quint/apalache-dist-0.62.4/apalache/bin/../lib/apalache.jar -> …/.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar
ok-cdpath exit 0
[ok] No violation found (3013ms).
violation: server pid 94635; executable …/.pixi/envs/default/lib/jvm/bin/java -> …/.pixi/envs/default/lib/jvm/bin/java; jar … -> …/.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar
violation exit 1
[violation] Found an issue (3055ms).
lsof -iTCP:8822 -sTCP:LISTEN exit 1
pgrep -fl apalache exit 1
archive=8ac8488422c30d552735bae11b92a2d014d8e02e8608702acc08f9a2d8a9abb2
Validated 49 pages and 50 canonical topics.
Validated AGENTS.md, 2 adapters, and 14 skills.
All checks passed and the worktree is unchanged.
Verification passed.
```

`…` stands for the worktree path. Each `refused` line follows a check of its own
diagnostic; the gate's mutation printed the pin check's own message
(`check-apalache-pin: tools/quint/apalache.txt says quint=0.33.0, …`). The port and process controls, which die
rather than print when they fail, ran before the first empty probe. Each live run logged
`Using existing Apalache distribution` and the `0.62.4` banner. `git status --short` was
empty at the end. Every run here was on macOS, so the Linux branch of the executable and
`-jar` reads was checked by `sh -n` only. The raw logs stay in `ai_tmp/`.

## Open points

- **Checking the tag in CI.** Gate 2 checks that the review was recorded, not that
  `apalache=` equals the CLI's tag. Adding `just install-quint` to the gate job would
  let CI check the tag too, at the cost of a network step in that job.
- **Where the pin lives.** The alternative to `tools/quint/apalache.txt` is a key in
  `tools/quint/package.json`, which npm ignores. It would keep one file per Quint bump,
  but Renovate's npm manager rewrites that file.
- **The evaluator.** The Rust evaluator for `run`, `test` and the REPL is still
  downloaded unchecked. This ticket's pattern would serve it, with one archive per
  platform.
- **Installing while verifying.** A `just quint verify` or `compile` that has passed the
  guard, while `just install-apalache` moves the tree aside, can find no launcher and
  download Apalache itself (checkpoint 1 C5/B4). The guarantee holds for non-concurrent
  use only, and the next guarded run refuses the unchecked tree. A lock across the guard
  and the whole CLI call would close it.
- **What the stamp proves.** It detects a local change against the installer's record,
  not a coordinated rewrite of the tree and the stamp together.
- **A network-denied Verification.** `verify.sh` runs no step under §11's denial; step
  6's run at `ffe512b` does (checkpoint 2, 2C1). Moving the installer's skip under
  `sandbox-exec` with both `curl` controls into Verification would put it in one script.
- **A `just` that ignores TERM.** `verify.sh`'s watchdog and trap stop Quint by killing
  `just`; one that ignored TERM would leave node running until the server's death ends
  it. Not observed and not probed.
- **Port 8822 and `--server-endpoint`.** A server already on `localhost:8822`, or the
  one `--server-endpoint` names, answers `verify` and `compile` with whatever Apalache it
  runs, and the guard cannot see which. Both lenses at checkpoint 1 recommended leaving
  them as documented caveats; AP01's Open points has the alternatives.
