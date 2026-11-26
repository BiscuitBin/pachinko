---
id: QM01
title: "The Rust evaluator pinned by checksum"
status: done
depends_on: []
parallel_with: []
branch: ticket/qm01-evaluator-pin
estimated_size: M
---

# QM01: The Rust evaluator pinned by checksum

## Context

The Quint CLI 0.34.0 is pinned by `tools/quint/package-lock.json` and Apalache 0.62.4 by
`tools/quint/apalache.txt`, with an installer, a gate and a guard in the `quint` recipe
(decision 0021, `docs/decisions/0021-project-managed-quint-cli.md`, and
`.scratch/apalache/AP02-checksummed-apalache.md`). One download is left that nothing
checks. Decision 0021 says so at lines 113 to 117: "The evaluator is still pinned by
version only, because the locked CLI names it, and verified by nothing beyond TLS".

The later tickets of this series read counterexamples, witnesses and traces through that
evaluator. This ticket closes the gap first, the way AP02 closed Apalache's.

What the CLI does was read in the installed package, under
`tools/quint/node_modules/@informalsystems/quint/dist/src/`. All of it is read, not run:

- `rust/binaryManager.js` line 25: `QUINT_EVALUATOR_VERSION = 'v0.8.0'`. The string
  carries a `v`.
- `config.js` lines 45 to 47: `rustEvaluatorDir` builds
  `<QUINT_HOME>/rust-evaluator-<version>`, so with the `Justfile`'s `QUINT_HOME` the
  directory is `.quint/rust-evaluator-v0.8.0/`.
- `rust/binaryManager.js` lines 32 to 46: `getRustEvaluatorPath` returns
  `<that directory>/quint_evaluator` when it exists, and otherwise downloads. "Exists"
  is a bare `stat` (lines 157 to 162): it follows a symlink and accepts a directory, and
  nothing reads the file's content or its mode.
- The download (lines 55 to 74, 112 to 125 and 197 to 232) prints
  `Fetching Rust evaluator v0.8.0...` at every verbosity (line 57), reads the release
  `evaluator/v0.8.0` of `quint-co/quint` through
  `https://api.github.com/repos/quint-co/quint/releases/tags/evaluator/v0.8.0` (line
  200), and fetches the asset by the API's `asset.url`. It never builds a direct
  download URL and never checks a hash. An archive already lying in the directory under
  the asset's name is used without a download (lines 119 to 122).
- One asset per platform (lines 126 to 156):
  `quint_evaluator-aarch64-apple-darwin.tar.gz` (line 132) and
  `quint_evaluator-x86_64-unknown-linux-gnu.tar.gz` (line 143) are the two for the
  platforms `pyproject.toml` line 16 lists, `osx-arm64` and `linux-64`. The archive is
  unpacked with `tar -xzf <archive> -C <directory>` and `chmod +x` on the executable
  (lines 88 to 91), which suggests `quint_evaluator` sits at the archive's root.
- Who starts it. `rust/commandWrapper.js` lines 219 to 226 call `getRustEvaluatorPath`
  with no argument for `simulate`, `test` and `evaluateAtState`;
  `rust/replServerWrapper.js` line 57 does the same for the REPL. No option and no
  environment variable changes the version. The child is spawned with three pipes and
  no shell (`rust/commandWrapper.js` lines 243 to 246), so the CLI talks to it over
  stdin and stdout, not over a socket. At `--verbosity=4` or above the CLI prints
  `Starting Rust evaluator with command: <command>` first (line 229; `verbosity.js`
  lines 95 to 97 and 105 to 109).
- Which commands reach it. `--backend` defaults to `rust` for the REPL (`cli.js` lines
  167 to 172), `test` (lines 218 to 223) and `run` (lines 310 to 315). `run` starts the
  evaluator when `backend` is `rust` (`cliCommands.js` lines 340 to 358) and evaluates in
  process otherwise (lines 360 to 364); `test` likewise (lines 245 to 256); the REPL at
  start-up (`repl.js` lines 110 to 111 and 192). The REPL is also the default command
  (`cli.js` line 143, `['repl [commands..]', '*']`). `verify` starts it when Apalache
  returns a counterexample: to name the violated invariants (`cliReporting.js` lines
  249 to 251, through `rust/invariantsReporter.js` lines 41 to 43) and to find which
  observation a trace reached (`verify.js` lines 248, 275 and 311). `parse`,
  `typecheck`, `docs` and `compile` (`cli.js` lines 134 to 139) never reach it.

Three things make a copy of AP02 by analogy wrong, and the design below answers each:

- **One archive per platform, and a version with a `v`.** Apalache has one archive and
  an `x.y.z` version. The evaluator's pin needs a hash per platform and a rule for a
  platform it does not list.
- **The allow-list inverts.** AP02's guard lets `run`, `test` and the REPL through
  unchecked, because they need no Apalache. They are exactly the commands that need the
  evaluator, and a leading option or an unknown word is a REPL.
- **The directory may not be empty, and the CLI trusts whatever is there.** A worktree
  that ever ran `just quint run` holds Quint's own unchecked download. An installer that
  skips because the executable exists would bless it.

AP02's reviews found defects that this ticket must not repeat. They are recorded in
`.scratch/apalache/AP02-checksummed-apalache.md`, hand-back notes 6 to 8: an option
matched by its spelling (C1/B1, C2/B2), `QUINT_HOME` pointing elsewhere (C3), a symlinked
directory (C4), a pipeline that hid a failed walk (C6/B3), `tar` environment variables
(round 2, B2), archive members checked after extraction (Departures, item 9), and a
Verification that accepted any non-zero exit (C7/B7).

Read these:

- `.scratch/quint-model/README.md`: the rulings, "Design points a ticket settles" and
  claims 1 to 4.
- `.scratch/apalache/AP02-checksummed-apalache.md`, in full, hand-back notes included.
- `scripts/install-apalache.sh`, `scripts/check-apalache-pin.sh` and
  `tools/quint/apalache.txt`: the pattern, with the fixes AP02's reviews forced.
- `Justfile`: the `QUINT_HOME` comment (lines 13 to 28), `install-quint`,
  `install-apalache`, `lock-check` and `quint`.
- `scripts/initialize.sh`, `renovate.json` (the rule whose `groupName` is `quint`),
  `.gitignore` and `.dockerignore`.
- `.github/workflows/ci.yml` and `.github/actions/setup/action.yml`: the `rust` job runs
  `just lock-check` after a setup that runs `just install-toolchain`,
  `just install-tools`, `just sync` and `just install-hooks`, and never
  `just install-quint`.
- The source files cited above, all under
  `tools/quint/node_modules/@informalsystems/quint/dist/src/`.
- `docs/decisions/0021-project-managed-quint-cli.md`, and
  `docs/how-to/maintain-dependencies.md`, "The Quint CLI".
- `tickets/CONVENTIONS.md` §11 and §14.

## Goal

`just initialize` installs the Rust evaluator into `.quint/rust-evaluator-<version>/`
from an archive whose SHA-256 this repository records for the platform it runs on. While
that install is in place, `just quint run`, `test`, the REPL and `verify` use it and
download nothing. `just quint` refuses each of them unless the stamped install is in
place and unchanged, so Quint never downloads the evaluator itself through the recipe,
and a binary that was changed after the install is refused before it runs. Gate 2,
`just lock-check`, fails offline, in CI as locally, on any move of the Quint CLI's pin
until the pin file records that the evaluator's pin was reviewed against the new CLI.

What stays open, and the pages say so: gate 2 checks that the review was recorded, not
that it was right; the stamp detects a local change, not a rewrite of the directory and
the stamp together; and the guarantee holds for non-concurrent use only. Open points
covers each.

### Owns

| Kind | Names |
| --- | --- |
| Rules | none |
| Invariants | none |
| Guarantees | none |
| Partial or extended, not counted | none: this ticket adds tooling and models nothing |

## Design proposed in this ticket

These are proposals. The maintainer confirms them, or changes them, after checkpoint 1 and
before step 4, and the hand-back records the ruling. After that the builder does not
reopen them. Later tickets are written against the recommended answers. If a ruling
differs, this ticket edits the sentences it changes in the later tickets and in the series
README, and lists them in the hand-back. The first four bullets answer the README's design
point "The layout of the evaluator's pin file, one hash per platform, and where the stamp
lives". The guard's two bullets answer "Which `just quint` commands need the evaluator,
and what the guard does with `--backend typescript`". The bullet on the proxy variables
answers no README row.

- **One pin file, `tools/quint/evaluator.txt`.** Comment lines start with `#`. Then
  `key=value` lines, each key once:

  ```text
  quint=0.34.0
  evaluator=v0.8.0
  sha256.linux-64=<64 lowercase hex digits, from step 2>
  sha256.osx-arm64=<64 lowercase hex digits, from step 2>
  ```

  `quint=` is the Quint CLI version the pin was reviewed against. `evaluator=` is the
  CLI's `QUINT_EVALUATOR_VERSION`, character for character, `v` included, so the
  directory name and the release tag are built from it with nothing added. Each
  `sha256.<platform>=` is the hash of that platform's release archive, and `<platform>`
  is a pixi platform name. The scripts read the file with `sed` and never source it.
  Alternatives: keys named by asset, or lines in `sha256sum` format, which would let a
  published checksum file be compared with `diff` but would not tie the list to pixi's;
  and `evaluator=0.8.0`, with the scripts adding the `v`.
- **The platforms are pixi's.** `scripts/check-quint-evaluator-pin.sh` reads
  `platforms` under `[tool.pixi.workspace]` in `pyproject.toml` and requires exactly one
  `sha256.<platform>=` line for each, and none for any other. Adding a platform to pixi
  then fails gate 2 until its hash is recorded. The installer and the guard map the
  machine to a platform with `uname -s` and `uname -m`: `Linux` and `x86_64` is
  `linux-64`, with the asset `quint_evaluator-x86_64-unknown-linux-gnu.tar.gz`; `Darwin`
  and `arm64` is `osx-arm64`, with `quint_evaluator-aarch64-apple-darwin.tar.gz`. On any
  other machine the installer fails, naming what `uname` printed and the platforms the
  pin lists, and installs nothing; the guard refuses the commands that need the
  evaluator the same way, and `typecheck` still runs. Alternative: a fixed list of two
  platforms in the script, which is simpler and lets pixi's list and the pin drift.
- **`scripts/check-quint-evaluator-pin.sh`, in `lock-check`.** Offline and read-only. It
  reads only `tools/quint/evaluator.txt`, `tools/quint/package.json`,
  `tools/quint/package-lock.json` and `pyproject.toml`, as they are in the checkout,
  never `tools/quint/node_modules/` or `.quint/`, so it fires in CI, where neither
  exists. It fails unless the file holds exactly the keys above beside blank and comment
  lines, `quint` is `x.y.z`, `evaluator` is `v` followed by `x.y.z`, each hash is 64
  lowercase hex digits, and `quint` equals both the `@informalsystems/quint` version
  `package.json` pins and the one `package-lock.json` locks. Every Quint bump therefore
  fails gate 2 until `quint=` is edited in both pin files. Every message starts
  `check-quint-evaluator-pin:`. JSON and TOML are read with pixi's `python -I -B`, as
  `scripts/check-apalache-pin.sh` reads JSON.
- **The stamp lives inside the directory it certifies.** The directory Quint looks in is
  the whole tree. The installer writes `.quint/rust-evaluator-<version>/.pachinko-stamp`
  with two lines: `archive=` the hash pinned for this platform, and `tree=` a digest of
  the directory. The digest is AP02's: the SHA-256 of the sorted `<sha256>  <path>`
  lines, each ending in a newline, of every regular file under the directory except the
  stamp, computed in one pixi `python -I -B` process that fails on any read error and on
  any entry that is not a regular file or a directory. The stamp certifies that the
  files there now are byte for byte the ones the installer unpacked from the archive
  with the pinned hash. The guard recomputes the digest on every run, so a changed
  binary, an added file and a leftover archive are each a mismatch. The executable is
  tested apart, because the digest covers content and not modes: `quint_evaluator` must
  be a regular file, not a symlink, and executable. Alternative: no stamp, and a
  `binary.<platform>=` hash of the executable in the pin file, which the guard compares
  directly; that pins a second hash per platform that no release publishes.
- **`scripts/install-quint-evaluator.sh`, run by `just install-quint-evaluator`.** POSIX
  `sh`, with the properties of `scripts/install-apalache.sh`. In order:

  1. It refuses to run from anywhere but the repository's root, and any `QUINT_HOME`
     other than `.quint`.
  2. It reads the pin file and fails if the pin check would. It maps the machine to a
     platform, as above.
  3. It fails, naming `just install-quint`, unless the installed package's `version` is
     the `quint` pin, and refuses a symlink anywhere on the path to the files it reads,
     as `check_cli` in `scripts/install-apalache.sh` does. It then fails, naming both
     values, unless `QUINT_EVALUATOR_VERSION` in `rust/binaryManager.js` is the
     `evaluator` pin, and fails unless that file names this platform's asset and the
     repository `quint-co/quint`. This is the one place the pin is compared with the
     CLI. A `sed` that matches nothing is a failure, not an empty value.
  4. It refuses a symlink or a non-directory at `.quint` or at the evaluator's
     directory, naming the recovery. It skips, with a line that starts
     `install-quint-evaluator:` and ends `nothing to do.`, only if the stamp is a
     readable regular file, its `archive=` is this platform's pinned hash, its `tree=`
     equals the digest recomputed now, and the executable passes its test. Any other
     directory there, Quint's own download included, is replaced.
  5. It downloads the archive with the flags `scripts/install-apalache.sh` line 333
     uses, into a new `.quint/install-quint-evaluator.XXXXXX` from `mktemp -d`, checks
     its SHA-256 (`sha256sum`, or `shasum -a 256` where that is missing), and on a
     mismatch prints both hashes, removes the temporary directory and exits non-zero.
     The URL is the one step 2 confirms (claim C).
  6. It lists the archive with `TAR_OPTIONS`, `TAR_READER_OPTIONS`,
     `TAR_WRITER_OPTIONS` and `TAPE` unset, refuses a listing with any member the plan
     does not allow (claim B fixes the rule; an absolute name, an empty, `.` or `..`
     component, a member that is not a regular file or a directory, and a listing
     without `quint_evaluator` are refused whatever it finds) before anything is
     unpacked, unpacks into the temporary directory with the same four variables unset,
     and requires the unpacked set to equal the listing. A member's type is read before
     unpacking, since a link unpacked first can send a later member outside the
     directory; the plan says how, from `tar -tvzf`'s first column or from pixi's Python
     `tarfile`, which lists without unpacking. It runs nothing from the archive.
  7. It writes the stamp into the unpacked directory and only then moves the old
     directory aside and the new one into place, then runs the skip test on the result.
     An interrupted run leaves the old directory or none, never an unstamped one that
     looks finished.
  8. Every `rm -rf` names its path through `"${var:?}"`, a trap removes the temporary
     directory on every exit, and the only path ever removed is that directory.
- **The guard: which commands need the evaluator.** The installer has a second mode,
  `--check -- <quint arguments>`, offline and read-only, and the `quint` recipe runs it
  as its second line, after `scripts/install-apalache.sh --check`. It runs the
  installer's check 1 for every command. For a command that needs the evaluator it then
  runs checks 2 to 4 and exits 0 only where check 4 would skip; otherwise it prints a
  line that starts `install-quint-evaluator --check: refused:`, gives the reason and
  names `just install-quint-evaluator`, and exits 1. It reads only the first argument,
  as AP02's guard does by the maintainer's ruling:

  | Arguments | Apalache, today | Evaluator, this ticket |
  | --- | --- | --- |
  | none (the REPL), or first `repl` | not needed | needed |
  | first `run` or `test` | not needed | needed |
  | first `parse`, `typecheck` or `docs` | not needed | not needed |
  | first `compile` | needed | not needed |
  | only `--help` or only `--version` | not needed | not needed |
  | anything else: `verify`, a leading option, an unknown word | needed | needed |

  So the evaluator is needed unless the first argument is `parse`, `typecheck`,
  `compile` or `docs`, or the only argument is `--help` or `--version`. `verify` is in
  the last row because the guard cannot know whether the run will report a violation
  (claim 1). A leading option and an unknown word are there because the CLI's default
  command is the REPL. `verify --help` is refused without the install; that fails
  closed, and the message says why. Alternative: demand the evaluator for every command
  that needs Apalache, `compile` included, which is one rule fewer and refuses a
  `compile` that never starts the evaluator.
- **The guard and `--backend typescript`.** README claim 4 leaves this to this ticket:
  "QM01 says whether the guard reads the option or fails closed on every `run`".
  Recommended: it fails closed, and does not read `--backend`. `run`, `test` and the
  REPL need the stamped install whatever backend they name, so
  `just quint run --backend typescript` is refused in a worktree that has not run
  `just install-quint-evaluator`, although that run would start no evaluator (claim 4).
  The reasons: the first-argument rule never parses an option's value, which is how
  AP02's checkpoint 1 dissolved `--target="tlaplus"` (C2/B2) after the CLI's parser was
  found to accept spellings a scan missed (C1/B1); the two errors are not alike, since a
  run wrongly let through downloads an unchecked binary and a run wrongly refused costs
  one install command; and `just initialize` installs the evaluator, so only a worktree
  that skipped it is refused. The refusal's message says that the backend is not read.
  Alternative: let a `run`, `test` or `repl` through without the install when exactly
  one argument is `--backend=typescript`, or `--backend` is followed at once by
  `typescript`, no other argument's option name contains `backend` in any case, and no
  argument is `--`. Under either answer the executing agent probes these spellings and
  records what the CLI does with each: `--backend=typescript`, `--backend typescript`,
  `--backend="typescript"`, `--no-backend`, `--BACKEND=typescript`, `--backend` given
  twice, and `-- --backend=typescript`. Step 6 shows what the guard does with each, and
  step 7 what the CLI does, where the stamped install lets every spelling reach it.
- **The proxy variables.** This point is the ticket's own; no README row names it. The
  CLI reaches Apalache over gRPC on `localhost:8822`, and its gRPC client sends even
  that connection through a proxy when `grpc_proxy`, `https_proxy` or `http_proxy` is
  set, unless `no_grpc_proxy` or `no_proxy` names the host. It reads those names in
  lower case only (`tools/quint/node_modules/@grpc/grpc-js/build/src/http_proxy.js`,
  lines 35 to 55 and 160 to 190; read, not run: claim G). A port probe on a raw socket
  ignores them, so with one set the probe and Quint do not talk to the same port, and
  a `verify` is answered by whatever the proxy reaches. Recommended: the `quint`
  recipe's `env` also unsets `grpc_proxy`, `http_proxy`, `https_proxy`, `GRPC_PROXY`,
  `HTTP_PROXY` and `HTTPS_PROXY`. Once this ticket has landed no `just quint` command
  needs the network: Apalache and the evaluator come from their own recipes, which
  keep the variables. The upper-case forms go as well, although the gRPC client does
  not read them, because nothing a `just quint` command starts has a use for a proxy
  and what else reads them was not read. Cost: a `verify` given a remote
  `--server-endpoint` that only a proxy can reach no longer works through the recipe.
  The guard does not refuse that option, and shared rule 10 forbids it in this series.
  Alternative: leave the recipe as it is. QM03's script then unsets the three
  lower-case names for its own runs (QM03, "Design proposed", point 4), and every
  by-hand `verify` before it, this ticket's and QM02's, unsets them on its own command
  line, apart from step 7's red run, which sets `grpc_proxy` on purpose.
- **`just initialize` runs `just install-quint-evaluator` after `just install-apalache`.**
  Both follow `just install-quint`, since each refuses a pin the installed CLI disagrees
  with.
- **CI does not install the evaluator in this ticket.** No CI job runs a command that
  needs it before QM03: QM02's gate, `check-quint`, runs `typecheck` only. Gate 2 runs
  the pin check in CI from committed files. QM03's `quint` job is the first to run
  `just install-quint-evaluator` there.
- **Renovate does not track the evaluator.** No manager reads
  `tools/quint/evaluator.txt`. The pin moves with a Quint bump, by hand, as Apalache's
  does, and the `quint` rule's description says its pull request fails `lock-check`
  until both pin files are reviewed.
- **The Apalache installer is not refactored.** The new script repeats the digest walk,
  the archive checks and the path rules of `scripts/install-apalache.sh` instead of
  moving them into a shared file. Alternative, under Open points: one shared file.

## Claims this ticket checks

Claims 1 to 4 keep the README's numbers. The lettered claims are this ticket's own; G
is the one the proxy variables rest on.
"Read" means read in the installed source and not run. An answer that differs from the
one below is a design change: **stop** and bring it back.

| # | Claim | Check |
| --- | --- | --- |
| 1 | A plain `verify` that passes does not start the Rust evaluator. A `verify` that names an invariant and reports a violation does, and so does an `--observation` that is reached. Read in `verify.js` lines 248 to 262, `cliReporting.js` lines 230 to 257 and `rust/invariantsReporter.js`. Read to its conditions: a trace starts the evaluator only when the run named an invariant, every name parsed as an expression (`verify.js` lines 191 to 196) and the trace has a last state (`cliReporting.js` lines 242 to 251), so a trace returned with no invariant named, as for a deadlock, prints `[violation]` and starts nothing | step 7: three `verify` runs at `--verbosity=4`, each log searched for `Starting Rust evaluator with command:`. They show the path of a named invariant and the path of an observation, and no other |
| 2 | The executable at `<QUINT_HOME>/rust-evaluator-<version>/quint_evaluator` is used when it exists and nothing is downloaded. The CLI checks that it exists, not what it is. QM01 runs this probe before it adds the guard, which would refuse a changed binary. Read in `rust/binaryManager.js` lines 32 to 46 and 157 to 162, and `config.js` lines 45 to 47 | first half: step 7, a `run` under the network denial. Second half: step 5's last case, a stand-in executable that the unguarded recipe runs |
| 3 | Whether the evaluator's releases publish checksums. The CLI's source cannot say; the release's asset list can. Not read | step 2: the release's asset list, and each asset's `digest` field; if neither exists, say so, and the pin rests on downloads over TLS |
| 4 | `run --backend typescript` evaluates in process and downloads nothing. QM01 says whether the guard reads the option or fails closed on every `run`. Read in `cliCommands.js` lines 340 to 364, and for a violation `cliReporting.js` lines 182 to 193 | step 5's last case, with the stand-in in place, and step 7 under the network denial. The guard's answer is under "Design proposed": it fails closed |
| A | The release `evaluator/v0.8.0` of `quint-co/quint` holds the two assets named in `rust/binaryManager.js` lines 132 and 143, one archive per platform. Not read | step 2: the asset list |
| B | Each archive holds `quint_evaluator` at its root. Inferred from `rust/binaryManager.js` lines 88 to 91; what else it holds is not read | step 2: `tar -tzf` and `tar -tvzf` on both archives; the plan fixes the member rule from the listings |
| C | A direct URL, `https://github.com/quint-co/quint/releases/download/evaluator/<version>/<asset>`, serves the asset the CLI would fetch through the API. Not read: the CLI only uses the API's `asset.url` | step 2: each asset's `browser_download_url` in the release's JSON, and the download's hash against the asset's `digest` |
| D | The CLI talks to the evaluator over pipes, so `just quint run` needs no loopback interface and passes under §11's full denial. Read in `rust/commandWrapper.js` lines 243 to 246 | step 7 |
| E | `uname -s` and `uname -m` name the platform that pixi's `node` reports as `process.platform` and `process.arch`, which is what the CLI reads (`rust/binaryManager.js` lines 34 and 35) | step 2, on the machine the ticket runs on |
| F | `Fetching Rust evaluator` is printed at every verbosity when the CLI downloads, and `Starting Rust evaluator with command:` at `--verbosity=4` when it starts the child. Read in `rust/binaryManager.js` line 57, `rust/commandWrapper.js` line 229 and `verbosity.js` lines 95 to 97 | step 7: the second is each silent probe's positive control, in the same log; the first is matched against the source line |
| G | With `grpc_proxy` set and no `no_proxy` for the host, the CLI's connection to `localhost:8822` goes to the proxy and not to the port. Read in `tools/quint/node_modules/@grpc/grpc-js/build/src/http_proxy.js` lines 35 to 55 and 160 to 190, not run; what a `verify` then prints is not read | step 7: a `verify` with `grpc_proxy` pointing at a loopback port where nothing listens, before the recipe is changed |

## Non-goals

- No `.qnt` module under `quint/`. The one module this ticket runs is a scratch file in
  `ai_tmp/qm01/`.
- No gate that runs Quint, and no CI job. `check-quint` is QM02's and the `quint` job is
  QM03's.
- No check of the installed CLI's version for `typecheck`. The guard leaves early for
  `typecheck`, before it compares the installed CLI with the pin, so QM02's gate, in
  the `scripts/check-quint.sh` that QM02 creates, checks that version itself. This
  ticket does not change that.
- No change to Apalache's pin, to `tools/quint/apalache.txt`, or to
  `scripts/install-apalache.sh` and `scripts/check-apalache-pin.sh`.
- No custom Renovate manager.
- No change to how Quint finds an Apalache server beyond the proxy variables of
  "Design proposed", and no refusal of `--server-endpoint`.
- Shared rules 6 and 11 ask nothing of this ticket: it has no instance, witness, mutant
  or fixture. Its vacuity controls are the tampered binary and the run with the network
  denied.
- No `CHANGELOG.md` entry. It records the crates' changes.

## Shared rules (QM01 to QM13)

1. **The Allium decides.** `docs/specs/pachinko.allium` is the source of truth for
   behaviour (decision 0002). The model is derived from it and from nothing else. Read
   each rule and invariant in the Allium at the ticket's base, never from a ticket's
   summary of it, a page or a chat. No ticket reads the Rust or the double to decide
   what the model does. A ticket may read them for names, for the walk's domains and
   configuration, and, in QM10 to QM12, to map steps and state.
2. **The model never leads.** A counterexample has one of three causes. The model
   transcribes the Allium wrongly: fix the model. The property transcribes the Allium
   wrongly: fix the property, against the Allium's text. The Allium is wrong or silent:
   stop, write the trace and the question under Open points, and ask the maintainer. Never
   weaken an invariant, narrow a domain, drop a rule or add a guard the Allium does not
   have to make a run pass. Never settle either `open question` in the Allium.
3. **Names are fixed.** The model is `quint/pachinko.qnt`, module `pachinko`. An
   instance is a module `pachinko_<bound>` at its foot; the first is `pachinko_walk`.
   The sum type `Rule` has one variant for each Allium rule the model has so far, spelt
   as the Allium spells it.
   A rule has `guard_<Rule>`, `effect_<Rule>` and `witness_<Rule>`. An Allium invariant
   or guarantee is a property under its Allium name; `FiredMatchesGuards` and
   `UnansweredStepChangesNothing` check the labels. The ghosts are `fired`, `last`,
   `answer` and `prev`. A mutant is `quint/mutants/<ticket>-<slug>.patch`. Top-level
   actions carry `WalkStep`'s names (`crates/libpachinko/tests/queue/walk.rs`) in lower
   camel case. The gate is `just check-quint`, the verify recipe is `just verify-quint`,
   and the trace fixtures live in `quint/traces/`.
4. **One action per cascade, and every step is total.** A pull with its `HoldRecord`
   steps is one action. A lease's end with its `Freed` rules is one action. A publish
   with the revival it causes is one action. A step that no rule answers changes
   nothing and leaves `fired` empty; from QM09, `advance` and a `tick` with nothing due
   move only the clock. No guard and no effect on the state reads a ghost.
5. **The domains are a ruling.** `pachinko_walk` has the walk's domains and
   configuration: 2 publishers, 3 consumers, 3 pockets, 3 record keys, ranks 0 to 3,
   pull limits -1 to 3 (-1 and 0 until QM05), `max_pull_batch` 2 and
   `max_consecutive_expiries` 2. A ticket
   that cannot verify at these inside the CI job's timeout stops and asks. It does not
   shrink a domain or drop an instance. QM03 sets each instance's `--max-steps`; a
   later ticket may raise it and never lowers it.
6. **Verified means this.** From QM03 on, `just verify-quint` shows all of it. `Canary`
   is violated on every instance. Every property passes on every instance, each named
   on the command line. Every rule the ticket owns has its witness reached. Every
   mutant the ticket lists typechecks and is caught by the one property it names, which
   may be any property the recipe checks, and which the unchanged model's pass on that
   instance shows to hold at the same bound. Each check fails closed: zero instances, a
   witness not reached, a patch that does not apply, a mutant that survives, a mutant
   caught by another property, and an error or a deadlock read as a catch are all
   failures. QM02 shows the same by hand for one witness and one mutant. An owned
   invariant that no listed mutant aims at is reported in the hand-back as checked and
   not mutation-tested.
7. **No clock before QM09.** QM02 to QM08 add no `now`, no timestamp field and no TTL.
   While the model has no clock, a specification change that only says when something
   is due changes nothing in the model, as long as the untimed lapse still covers it.
8. **What the model lacks is written down.** A behaviour the Allium allows and the
   model does not yet have is one line in the model's header comment, under "Not yet
   modelled", naming the ticket that adds it. That ticket deletes the line. A property
   that would fail only because of such a gap carries the gap as a named exception, and
   the same ticket removes the exception. A property is transcribed whole, conditions
   included, even where a condition is always true so far.
9. **Quint runs through the recipe.** Every Quint command is `just quint <args>`, run
   from the repository's root, never a bare `quint` on `PATH`. A claim about the CLI's
   behaviour is checked by running the pinned CLI and quoting its output, never from
   memory or from a page.
10. **The network is named, and the checker is the pinned one.** `just initialize`,
    `just install-quint`, `just install-apalache` and `just install-quint-evaluator`
    reach the network, and each is a named, authorised step in the ticket that runs it
    (`tickets/CONVENTIONS.md` §11, "Network"). `just check` reaches none. No step lets
    Quint download anything itself. Before any `verify`, show that nothing is listening
    on `localhost:8822`; never pass `--server-endpoint`; never run a `verify`, or any
    other `just quint` command that needs the evaluator, while `just install-apalache`
    or `just install-quint-evaluator` runs or while another worktree verifies. A `verify` talks
    to its own server over the loopback interface, so a network denial used as evidence
    for a `verify` denies the external network and allows loopback, and its negative
    controls are §11's two `curl` commands.
11. **One change carries the model and what depends on it.** A change to
    `quint/pachinko.qnt` re-cuts every mutant patch it breaks, keeping the mutation and
    the property it names, and runs `just verify-quint` once QM03 has landed that recipe.
    Once `quint/traces/` exists it also regenerates the fixtures, before any gate runs,
    because `just check-quint` then compares the model's hash, and keeps both replays
    green: the Rust test in `just test` and, once QM12 has landed, `just python-double`. A
    field or a step the model gains is either mapped in each replay or added to that
    replay's list of what it does not compare, naming the ticket that will map it. A step
    on a replay's list ends that replay of every trace that holds it, so a ticket lists a
    step only with the maintainer's answer on which ticket maps it. The patches, the
    fixtures, their generator and the replays are in the Files touched of every ticket
    that changes the model, and the hand-back lists each patch re-cut. A change to
    `docs/specs/pachinko.allium` that touches a modelled rule, invariant or guarantee
    updates the model in the same change, runs `just check-quint`, and runs
    `just verify-quint` if the recipe exists.
12. **Files.** Edit only the files the ticket lists, plus its own `status:` line and
    hand-back notes. A shared file is listed as `shared, own lines`. A change needed
    elsewhere is handed back, not made (`tickets/CONVENTIONS.md` §11). Scratch goes in
    `ai_tmp/<ticket>/` and is never committed.
13. **Evidence, and the series base.** Each quoted output is headed by the commit it ran
    at and shows its exit status. A check that passes by printing nothing follows its
    positive control (`tickets/CONVENTIONS.md` §11, "Quoted evidence names its commit"
    and "Checks fail closed"). Where §11 says `main`, read the series base the
    maintainer named: for the worktree's start, for the closing fetch and merge, and
    for the evidence quoted afterwards. Pushing and opening a pull request are
    separately authorised.

## Adversarial review

This ticket runs `tickets/CONVENTIONS.md` §14 at each checkpoint it marks: its phases,
its two lenses, its two rounds at most, its scope guard and its record. These deltas
apply, and every ticket of the series carries them word for word.

- **Reviewer.** Codex, Sol, which is `codex exec -m gpt-6.2-sol`. The maintainer named
  Sol for this series on 2026-11-25, in place of the model §14 prints. Confirm
  `codex exec --help` on the day, as §14 asks. Fall back to a fresh-context
  `claude-opus-5-5` subagent only if `codex` is not on `PATH`, and say so in the
  hand-back notes.
- **A usage limit is not a clean review.** If a Codex log ends in a usage-limit error,
  the lens is incomplete. Stop and ask the maintainer whether to wait or to use the
  fallback. Never record it as "no findings".
- **Command.** Run this from the worktree, once per lens:

  ```sh
  dir="$PWD/ai_tmp/review/<id>-<n>"
  mkdir -p "$dir"
  codex exec -m gpt-6.2-sol -s read-only -C "$PWD" \
    -o "$dir/last-<lens>.md" "<brief>" \
    < /dev/null >| "$dir/log-<lens>.txt" 2>&1 &
  pid=$!
  wait "$pid"
  echo "exit $?"
  ```

  `<id>` is the ticket id, `<n>` the checkpoint's number, and `<lens>` the lens. A
  second round uses `<lens>-2` in all three names. `< /dev/null` closes stdin; without
  it the run waits forever. `-o` holds only the last message. `>|` is for a shell with
  `noclobber`. Under `-s read-only` Codex cannot write files, so the brief tells it to
  put its complete findings in its final message, and the runner copies
  `last-<lens>.md` to `findings-<lens>.md` under a first line `Source: last-message`.
  If §14 on the ticket's base already prints a command that redirects stdin from
  `/dev/null`, use §14's, with Sol's model.
- **Completion.** A lens is complete when its process, followed by the process id the
  command captured, has exited 0 and `last-<lens>.md` is not empty. Never wait on a
  file name or a `pgrep` pattern, which can match a run in another worktree. A lens
  that exits non-zero or leaves an empty file runs again, once; a second failure stops
  the ticket.
- **Lenses.** `conformance` and `breakage`, as §14 defines them. A checkpoint marked
  "with a design lens" adds a third reviewer, `design`: does the model say what the
  Allium says, is anything the Allium allows missing without being listed under "Not
  yet modelled", and could a property hold for a reason other than the one claimed.
- **Brief.** It gives the ticket's path, `tickets/CONVENTIONS.md`,
  `docs/specs/pachinko.allium`, the checkpoint's questions, and the artefact. At a plan
  checkpoint the artefact is the plan file. At a checkpoint on a model or a recipe it
  is the files the ticket names there, with the output of the controls run so far. At
  a diff checkpoint it is `git diff "$base"...HEAD` plus the uncommitted changes.
  `base` is the commit step 1 records. The brief never gives the author's reasoning or
  the hand-back's conclusions.
- **Triage and apply.** Separate fresh subagents check each finding by a command or a
  read and write `ai_tmp/review/<id>-<n>/verdicts.md`. They apply valid fixes one
  writer at a time. Before the lead runs `just check` or `just verify-quint`, it
  confirms every writer has stopped.
- **After the last checkpoint.** A merge or a rebase onto the series base, or a fix made
  after the last checkpoint, as after a red CI run, that changes a file under the
  ticket's Files touched was not reviewed. The agent shows that diff
  at the push stop and asks whether the last checkpoint runs over it first.
- **Network.** Each checkpoint authorises its model call (§11, "Network").

`<id>` is `QM01`. Checkpoints:

- Checkpoint 1, the plan, `ai_tmp/qm01/plan.md`, before any tracked file changes: step 3.
- Checkpoint 2, the diff with the pages included, before the final commit: step 9.

## Files touched

Each page row covers every sentence on that page that says the evaluator is downloaded
on first use, is checked by nothing, or lists what `just initialize`, gate 2 or the
network recipes do. Each such sentence is brought into line, keeping the gaps under Goal
stated. Find them with
`rg -n -i 'evaluator' docs AGENTS.md Justfile scripts renovate.json .gitignore .dockerignore`;
the line numbers below are as at commit `cf1f3f3`.

| Path | Class | Content |
| --- | --- | --- |
| `tools/quint/evaluator.txt` | new | the pin file, with a comment naming decision 0021 and the bump how-to |
| `scripts/install-quint-evaluator.sh` | new | the installer, POSIX `sh`, with the `--check` mode the guard runs |
| `scripts/check-quint-evaluator-pin.sh` | new | gate 2's check, POSIX `sh` |
| `Justfile` | shared, own lines | `install-quint-evaluator`, with a comment; a line in `lock-check`, and its comment; the guard's second line in the `quint` recipe, and its comment (line 472); under the recommended ruling on the proxy variables, six more `-u` flags on that recipe's `env` (`grpc_proxy`, `http_proxy`, `https_proxy` and their upper-case forms) and one sentence in its comment saying that the gRPC client would send the loopback connection through a proxy and that no `just quint` command needs one; the `QUINT_HOME` comment (lines 13 to 27) |
| `scripts/initialize.sh` | shared, own lines | `just install-quint-evaluator` after `just install-apalache`, with a comment; the comment over `just install-quint` (lines 39 to 42) |
| `renovate.json` | shared, own lines | the `quint` rule's description (line 84) |
| `.gitignore` | shared, own lines | the comment over `/.quint/` (lines 70 to 74) |
| `.dockerignore` | shared, own lines | the comment over `.quint/` (lines 64 to 66), which still says Quint downloads the evaluator and Apalache |
| `AGENTS.md` | shared, own lines | line 37: `just install-quint-evaluator` among the recipes that reach the network, and the clause on what `just quint` downloads |
| `docs/decisions/0021-project-managed-quint-cli.md` | shared, own lines | a note dated the day this lands, in Consequences in place of lines 113 to 117: the evaluator is pinned by checksum, with the gaps under Goal. Decision: bullets for the pin file, the installer, gate 2 and the guard's table. Context lines 24 to 30 stay as the CLI's own behaviour. What would reopen this, lines 139 to 140 |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | "The Quint CLI": the sentence at lines 316 to 319, a subsection "Moving the evaluator pin" (read the constant, download each platform's archive, hash, edit, install), and the Renovate paragraph |
| `docs/how-to/develop-locally.md` | shared, own lines | "Quint", lines 167 to 178, and what `just initialize` installs (lines 89 and 93) |
| `docs/explanation/security-model.md` | shared, own lines | the Quint CLI exception, lines 69 to 73: the evaluator is checked when installed by the recipe, with the gaps under Goal |
| `docs/reference/commands.md` | shared, own lines | an `install-quint-evaluator` row; the network paragraph at lines 16 to 24, with its counts "Twelve" and "the first seven"; the `initialize`, `lock-check` and `quint` rows |
| `docs/reference/quality-gates.md` | shared, own lines | gate 2's row (line 20), its repair row (line 171), and the recipes that reach the network (line 189) |
| `docs/reference/configuration.md` | shared, own lines | the `QUINT_HOME` row (line 69), a row for `tools/quint/evaluator.txt`, and the pins table (line 218) |
| `docs/operations/maintenance.md` | shared, own lines | the sentence on moving the Quint CLI's pin (lines 40 to 43) |
| `docs/project/repository-map.md` | shared, own lines | the `tools/quint/` line (89) names the pin file; the `scripts/` row (line 114) names the two scripts |
| `docs/tutorials/first-change.md` | shared, own lines | line 33, only if its list of what `just initialize` installs needs the evaluator |
| `docs/reference/testing.md` | shared, own lines | line 429, only if its sentence on `lock-check` lists what gate 2 checks |
| `.scratch/quint-model/QM01-evaluator-pin.md` | ticket | `status:`, the hand-back notes, and a correction to Verification that a control shows is needed, recorded under Departures |
| `.scratch/quint-model/README.md` | shared, own lines | QM01's Status cell in the index; and, only if the ruling at step 4 differs from a recommended answer, the sentences it changes there |
| `.scratch/quint-model/QM02-*.md` to `QM13-*.md` | shared, own lines | only if the ruling at step 4 differs from a recommended answer: the sentences it changes, each listed in the hand-back |

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/qm01-evaluator-pin` from
   the series base the maintainer names (README, "Before QM01 starts"), and record that
   base's full commit id as `base`. `git ls-files .scratch/quint-model/` must list the
   README and this ticket. Run `just initialize` (network, authorised). Run `just check`
   on the untouched worktree and quote its last line, headed by the commit. Then record
   `ls .quint`: a directory `rust-evaluator-*` there is Quint's own unchecked download
   from an earlier run. Leave it; it is the first thing step 5's installer replaces.
2. **Read before writing** (network, authorised: one read of a public release, two
   archive downloads and, only if the release lists a checksum asset, one download of
   that asset; no Quint command runs in this step).

   - Confirm the Context's reads with `grep -n` for `QUINT_EVALUATOR_VERSION`,
     `releases/tags/evaluator/` and `quint_evaluator-` in `rust/binaryManager.js`, and
     quote the lines.
   - Read the release without a credential, with `curl -fsSL --proto '=https' --tlsv1.2`
     and `-o ai_tmp/qm01/release.json`, from
     `https://api.github.com/repos/quint-co/quint/releases/tags/evaluator/v0.8.0`.
     Any HTTP error stops the step. Record the asset names (claim A), any checksum
     asset (claim 3), and each of the two assets' `digest` and `browser_download_url`
     (claims 3 and C).
   - Download both platforms' archives, each from its asset's `browser_download_url`
     with the flags `scripts/install-apalache.sh` line 333 uses, into new, empty
     directories `ai_tmp/qm01/download/linux-64/` and
     `ai_tmp/qm01/download/osx-arm64/`. Hash each
     with both `sha256sum` and `shasum -a 256`, and compare with the asset's `digest`.
     If the release lists a checksum asset, download it the same way into a new, empty
     `ai_tmp/qm01/download/checksums/` and compare each hash with it as well. Any other
     request is outside this step: **stop** and ask. Never copy a hash from this ticket,
     a page or a chat: there is none here to copy.
   - List both archives and quote the listings (claim B): `tar -tzf` for the names and
     `tar -tvzf` for each member's type. Every `tar` in this step runs under
     `env -u TAR_OPTIONS -u TAR_READER_OPTIONS -u TAR_WRITER_OPTIONS -u TAPE`, as
     `scripts/install-apalache.sh` runs its own. Before anything is unpacked, **stop**
     if a listing holds an absolute name, an empty, `.` or `..` component, or a member
     that is not a regular file or a directory. Only then unpack this machine's archive
     into a new, empty `ai_tmp/qm01/fixture/`. It stands for Quint's own download in
     step 5, and nothing in it is executed.
   - Quote `uname -s`, `uname -m` and
     `.pixi/envs/default/bin/node -p 'process.platform + " " + process.arch'` (claim E).
   - Write `ai_tmp/qm01/smoke.qnt`:

     ```quint
     module smoke {
       var x: int
       action init = x' = 0
       action step = x' = (x + 1) % 3
       val inRange = x >= 0 and x < 3
       val belowTwo = x < 2
       val reachedTwo = x == 2
     }
     ```

   - Write `ai_tmp/qm01/plan.md`: the pin file's content; each script's outline against
     "Design proposed"; the archive member rule, from the listings; the guard's table;
     the URL; and the edit to each page in Files touched.

   If an asset is missing, an archive holds no `quint_evaluator`, or no URL serves an
   asset without the API, **stop** and ask: the design changes.
3. **Adversarial review** (checkpoint 1, `tickets/CONVENTIONS.md` §14: conformance and
   breakage, over `ai_tmp/qm01/plan.md`, before any tracked file changes). Questions:

   - Can `check-quint-evaluator-pin.sh` pass in CI while the Quint pin and `quint=`
     disagree, or with a pixi platform that has no hash? Does it read anything CI does
     not have?
   - Can the installer or the guard accept a directory the installer did not unpack and
     check: Quint's own download, a directory repopulated after it was removed, one with
     the binary changed, a symlink at any of the paths, or a leftover archive?
   - Name an argument list for which the guard exits 0 and the CLI then starts or
     downloads the evaluator. Consider a leading option, an unknown first word, the
     default command, and every spelling of `--backend`.
   - Can the pin's `evaluator=` and the installed CLI's constant disagree while the
     guard passes?
   - Does any step let Quint reach the network itself, or run a `verify` against a
     server it did not start?
4. **Red: the gate.** **Stop** first for the maintainer's ruling on "Design proposed",
   with the checkpoint's findings in hand, and record it. If it differs from a
   recommended answer, edit the sentences it changes in the later tickets and in the
   series README before going on, and list each in the hand-back. Then add
   `tools/quint/evaluator.txt`, `scripts/check-quint-evaluator-pin.sh` and the
   `lock-check` line. Save the pin file to `ai_tmp/qm01/evaluator.txt.saved`. Show
   `just lock-check` passing. Show it passing again with `tools/quint/node_modules` and
   `.quint` each renamed into `ai_tmp/qm01/`, and rename them back: the gate reads
   neither, as in CI. Then, each on an edit restored from the saved copy afterwards, show
   `just lock-check` exiting 1 and quote each message: `quint=` changed to `0.33.0`; one
   `sha256.` line removed; a hash given 63 digits; a line `sha256.osx-64=` added with a
   well-formed hash; `evaluator=0.8.0`, without the `v`; a key `apalache=` added. After
   the last restore, `cmp tools/quint/evaluator.txt ai_tmp/qm01/evaluator.txt.saved`
   exits 0. Last, run the script under `dash`, with pixi's `bin` first on `PATH`, once
   passing and once on the `quint=0.33.0` edit: CI's `sh` is `dash`, and a construct
   only another shell accepts would pass every control on macOS and fail there.
5. **The installer.** Add `scripts/install-quint-evaluator.sh`,
   `just install-quint-evaluator` and the line in `scripts/initialize.sh`. The `quint`
   recipe is not changed yet. Then, quoting each run's output and exit status, and
   restoring each edit afterwards:

   - if step 1 found no evaluator directory, copy `ai_tmp/qm01/fixture/` to
     `.quint/rust-evaluator-v0.8.0/`, as Quint would leave it.
     `just install-quint-evaluator` (network, authorised) replaces the unstamped
     directory and writes the stamp;
   - a second run skips and says so, and so does the script run under `dash` with
     `QUINT_HOME=.quint` and pixi's `bin` first on `PATH`;
   - append one byte to the installed `quint_evaluator`. The installer names the digest
     mismatch and replaces the directory (network, authorised);
   - add a file named as this platform's asset beside the executable. The installer
     replaces the directory (network, authorised);
   - remove the directory, copy the fixture back in, and the installer replaces it,
     because the stamp went with the removed directory (network, authorised);
   - `chmod a-x` the installed `quint_evaluator`. The installer replaces the directory
     (network, authorised);
   - put a symlink at `.quint/rust-evaluator-v0.8.0`, pointing at a copy of the
     directory under `ai_tmp/qm01/`. The installer refuses, names the recovery, and
     changes nothing in the copy;
   - with `evaluator=v0.7.0` (the pin file otherwise unchanged, so the gate's check
     passes), the installer fails and names both versions;
   - with the installed package's `package.json` `version` edited to `0.33.0` (save the
     file under `ai_tmp/qm01/` first, restore it, and show `cmp` of the two exits 0),
     the installer fails and names `just install-quint`;
   - with `QUINT_HOME=ai_tmp/qm01/elsewhere` on `just`'s command line, the installer
     fails and creates nothing there;
   - with this platform's hash changed by one digit and the directory removed, the
     installer (network, authorised) fails, prints both hashes, and leaves no evaluator
     directory and no `.quint/install-quint-evaluator.*`;
   - the archive member check, run on its own on the input the installer gives it,
     which the plan fixed with the way a member's type is read: the `tar -tvzf` listing
     on standard input, or an archive opened with `tarfile` (AP02 fed its check a
     names-only listing; that cannot say a member is a link). An input with a `..`
     component, one with an absolute name, one with a symbolic link named as a regular
     member could be, and one without `quint_evaluator` are each refused, in the mode
     that runs before anything is unpacked. For `tarfile` the inputs are small
     archives built under `ai_tmp/qm01/`, none of them unpacked;
   - with a `uname` earlier on `PATH` that prints another machine, the installer fails,
     naming it and the pinned platforms. If pixi's environment holds a `uname` of its
     own, which the `Justfile` puts first, record that this case could not be run;
   - restore the pin file and run `just install-quint-evaluator` once more (network,
     authorised), so a stamped install is in place.

   Last, the one unguarded run of this ticket. It is the probe README claim 2 names,
   "QM01 runs this probe before it adds the guard, which would refuse a changed binary",
   and it is the red test for the guard and the strict form of claim 4. It runs here,
   after the installer and before step 6 adds the guard, and nowhere else. Shared rule
   10 holds, since nothing can be downloaded: the executable's path is occupied, and
   every command runs under §11's full denial, after both of §11's `curl` controls have
   failed inside the same denial and `true` has exited 0 inside it. Save
   `quint_evaluator` to
   `ai_tmp/qm01/binary.saved` and put in its place an executable `sh` script that
   creates `ai_tmp/qm01/sentinel-ran` and exits 3. Run the script once by hand to show
   the file appears, and remove the file. Then:

   - `just quint run --max-steps=3 ai_tmp/qm01/smoke.qnt` under the denial. Quote its
     output and exit status. Claim 2 expects the sentinel file to appear, an error that
     names exit code 3, and no `Fetching Rust evaluator` line: the CLI ran what it
     found. Remove the sentinel file;
   - `just quint run --backend typescript --max-steps=3 ai_tmp/qm01/smoke.qnt` under the
     denial. Claim 4 expects exit 0 and no sentinel file;
   - restore the binary from the saved copy, and show `just install-quint-evaluator`
     skipping, which proves the restore.

   If either run comes out otherwise, **stop**: claim 2 or claim 4 is wrong, and the
   guard's design rests on them.
6. **The guard.** Add the second guard line to the `quint` recipe, with its comment.
   Every refusal below must exit exactly 1, print
   `install-quint-evaluator --check: refused:` with its own reason, print no
   `Fetching Rust evaluator` line, and leave `ls -la .quint .quint/rust-evaluator-v0.8.0`
   as it was. The cases with no file argument run the guard directly, as the `guard`
   function in Verification does, so that no REPL starts. Quote each:

   - with the stamp moved to `ai_tmp/qm01/stamp.saved`: `just quint run`, `test` and
     `verify` on the scratch module are refused; the guard run directly is refused with
     no arguments, with `repl`, with `--verbosity=4` and with `nonsense`; and
     `just quint typecheck`, `parse`, `compile --target json` and `--version` pass;
   - still without the stamp, every `--backend` spelling listed under "Design proposed",
     after `run`, gives the result the ruling says. Under the recommended answer each is
     refused. Under the alternative, record for each spelling what the guard did, and
     for each one it let through, that the run under §11's denial exited 0 and started
     no evaluator;
   - with the stamp restored (`cmp` exits 0) and the stand-in script of step 5 in place
     of the binary, `just quint run` is refused for a digest mismatch and the sentinel
     file does not appear. Restore the binary and show the guard passing;
   - with one byte appended to the binary, and with `chmod a-x`, each refused; restore;
   - with a symlink at `.quint/rust-evaluator-v0.8.0`, as in step 5, refused;
   - with `evaluator=v0.7.0` in the pin file, refused, naming both versions; restore;
   - with `QUINT_HOME=ai_tmp/qm01/elsewhere`, the guard run directly is refused;
   - with everything restored, `just quint run --max-steps=3 ai_tmp/qm01/smoke.qnt`
     exits 0, and the guard run under `dash` with `run` exits 0.
7. **Quint uses the checked copy.** With the stamped install in place, and quoting each
   log's matching lines and each exit status:

   - under §11's denial (macOS:
     `sandbox-exec -p '(version 1)(allow default)(deny network*)'`; Linux:
     `unshare -rn`), with both `curl` controls failing and `true` passing inside the
     same denial: `just quint run --verbosity=4 --max-steps=3 ai_tmp/qm01/smoke.qnt`
     exits 0, logs `Starting Rust evaluator with command:` and no
     `Fetching Rust evaluator` (claims 2, D and F). The denial is the full one, with no
     loopback exception, because the evaluator is a child on pipes;
   - under the same denial, the same run with `--backend typescript` exits 0 and logs no
     `Starting Rust evaluator with command:` (claim 4);
   - under the same denial and at `--verbosity=4`, one run for each `--backend` spelling
     listed under "Design proposed", placed after `run`. Quote each exit status, and
     say for each whether its log holds `Starting Rust evaluator with command:`. The
     guard passes every one of them here, because the stamped install is in place, so
     this is where the CLI's own reading of each spelling is recorded, whatever the
     ruling;
   - `printf '1 + 1\n'` piped into `just quint`, which runs under the same denial: quote
     what the REPL prints and its exit status;
   - then the proxy variables ("Design proposed", as ruled; claim G), not under a
     denial, with nothing listening on `localhost:8822` before each run, shown as in
     the next bullet. Red, with the recipe as step 6 left it: show that nothing listens
     on `127.0.0.1:9`, then run this, which must not print `[ok] No violation found`:

     ```sh
     env -u no_grpc_proxy -u no_proxy grpc_proxy=http://127.0.0.1:9 \
       just quint verify --verbosity=4 --invariant=inRange --max-steps=5 \
       ai_tmp/qm01/smoke.qnt
     ```

     `no_grpc_proxy` and `no_proxy` are unset for it because either, naming
     `localhost`, makes the client skip the proxy (the same file as claim G's, lines 97
     to 105 and 166 to 177), and the run would then pass for that reason.

     Quote its exit status, its last lines and how long it took: the client may retry
     for a minute. If it passes, claim G is wrong: **stop**. Under the recommended
     ruling, add the six `-u` flags and the comment's sentence to the recipe. Green:
     the same command, `env -u` of the two names included, and once more with all six
     names set to that address, exits 0 and prints `[ok] No violation found`.
     Under the alternative, change nothing in the recipe, and run every other `verify`
     of this ticket, the three below and Verification's, under
     `env -u grpc_proxy -u https_proxy -u http_proxy`. The red run above is then the
     one `verify` with `grpc_proxy` set;
   - then the three `verify` runs of claim 1, not under a denial, since a `verify`
     talks to its own Apalache server over loopback. Before each, show nothing listens
     on `localhost:8822`, with the probe and its positive control from Verification.
     Pass no `--server-endpoint`, and run none while `just install-apalache` or
     `just install-quint-evaluator` runs or another worktree verifies (shared rule 10).
     `--invariant=inRange` exits 0 and logs no
     `Starting Rust evaluator with command:`. `--invariant=belowTwo` exits 1, logs
     `[violation] Found an issue`,
     `Starting Rust evaluator with command: evaluate-at-state-from-stdin` and the line
     that names `belowTwo` under the cross mark U+274C (`cliReporting.js` line 171).
     `--observation=reachedTwo` exits 0 and logs the same start line and
     `[observed] Observed reachedTwo, which stopped the run` (`cliReporting.js` line
     115). The CLI prints the start line before it spawns the child
     (`rust/commandWrapper.js` lines 229 and 243), so that line shows an attempt; the
     cross-mark line and the `[observed]` line are what the evaluator's answer prints,
     and each run needs its own. No log holds `Fetching Rust evaluator`, and afterwards
     the guard, run directly with `run`, still exits 0.
8. **Words.** Edit each page in Files touched. Run `just check-docs` and
   `just check-agents`.
9. **Adversarial review** (checkpoint 2, `tickets/CONVENTIONS.md` §14: conformance and
   breakage, over `git diff "$base"...HEAD` plus the uncommitted changes, with the pages
   included, before the final commit). Questions:

   - Would gate 2 fail on a Renovate pull request that moves the Quint CLI and touches
     nothing else? Would it fail in CI, where `tools/quint/node_modules/` is absent?
   - Could Verification pass while Quint downloaded the evaluator itself, or while the
     guard accepted a binary the installer did not unpack?
   - Does a refusal in Verification match the guard's own diagnostic, or would any
     failure satisfy it?
   - Do `lock-check` and the guard stay offline and read-only, and does `just check`
     stay free of the network?
   - Does every page in Files touched agree on where the pin lives, what checks it,
     which commands the guard covers and the gaps under Goal? Does any file outside
     Files touched still say the evaluator is unchecked?
10. **Commit and hand back.** Commit the work. Run `git fetch origin` (network,
    authorised), merge or rebase onto the series base, rerun `just check`, and rerun
    Verification and quote it, headed by the full commit id. If the merge or rebase
    changed a file under Files touched, checkpoint 2 did not see that change: show its
    diff at the stop and ask whether it is reviewed before the push (the review deltas,
    "After the last checkpoint"). Set `status: done` here and in the index of
    `.scratch/quint-model/README.md`, write the hand-back notes, and commit the record.
    Then run `just check` once more, on the record's commit: the Verification quoted in
    the notes ran on the commit before, and the hook gate has not yet read the notes.
    Show its last line and an empty `git status --short` at the stop. If it fails, fix
    the notes, commit and run it again; never amend the work's commit.
    **Stop** before pushing or opening a pull request: both need authorisation
    (`tickets/CONVENTIONS.md` §11, "Separately authorised actions").

## Acceptance criteria

- `tools/quint/evaluator.txt` names `quint=0.34.0`, `evaluator=v0.8.0` and one hash for
  each platform in `pyproject.toml`, each the one step 2 computed with two tools, and
  each of step 4's six mutations makes `just lock-check` exit 1 with a
  `check-quint-evaluator-pin:` message, each quoted.
- `just lock-check` passes with `tools/quint/node_modules` and `.quint` absent, quoted.
- The pin check, the installer's skip and the guard each run under `dash`, quoted.
- `just install-quint-evaluator` replaces an unstamped directory, a changed binary, a
  directory with an added file, a repopulated directory and a binary that is not
  executable; skips a stamped one; and refuses a symlinked directory, a mismatched
  constant, a mismatched CLI, another `QUINT_HOME` and a wrong hash, leaving no
  directory and no temporary directory after the last. Its member check refuses a
  listing with a `..` component, an absolute name, a symbolic link or no
  `quint_evaluator`, before anything is unpacked. Each is quoted from step 5.
- `just initialize` runs `just install-quint-evaluator` after `just install-apalache`.
- Before the guard, the unguarded recipe ran a stand-in executable under the network
  denial (step 5). After it, `just quint` refuses the same stand-in, and the sentinel
  file does not appear (step 6).
- `just quint` refuses `run`, `test`, `verify`, the REPL in both forms, a leading option
  and an unknown word without the stamped install; runs `typecheck`, `parse`,
  `compile --target json` and `--version` without it; and gives each `--backend`
  spelling the result the ruling says. Each refusal exits 1 with the guard's own
  diagnostic and changes nothing under `.quint/`, each quoted.
- `just quint run` exits 0 under §11's denial, with both `curl` controls failing and
  `true` passing inside it, and its log holds the evaluator's start line and no fetch
  line.
- With `grpc_proxy` pointing at a port where nothing listens, a `verify` through the
  recipe failed before the recipe unset the proxy variables and passes after, each
  quoted; or, under the alternative ruling, the failing run is quoted and every other
  `verify` of this ticket ran with the three lower-case names unset.
- Each of claims 1 to 4 and A to G has its outcome recorded, with the output it rests
  on. For claim 1 that is the violating run's cross-mark line and the observing run's
  exit status 0 and `[observed]` line, not the start line alone.
- `AGENTS.md` lists `just install-quint-evaluator` among the recipes that reach the
  network, and `just check-agents` passes.
- The pages state the gaps under Goal, and `just check-docs` passes.
- The hand-back says which controls ran on one platform only, and names it.
- Both checkpoints are recorded as §14's Record asks, with the review deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit, and both hold again on the
  record's commit.

## Verification

Corrected at step 4 under the maintainer's rulings (hand-back §9, item 6): each
refusal checks its own reason, an absent marker needs `grep`'s status 1, the port
control names its listener's PID, and the `curl` controls check exit codes after
positive runs outside the denial. Each `verify` that reaches the CLI (claim 1's three
and the dead-proxy run) runs under a denial that allows loopback only, and its log
shows Quint started its own Apalache and downloaded nothing; the `verify` the guard
refuses without the stamp runs under the full denial and starts nothing. The loopback
profile is macOS's, probed at step 7, so the script runs on Darwin only; Linux's
`unshare -rn` starts with `lo` down and was not probed. Run from a file as one `sh` script, from the repository's root, after the commit, with
the stamped installs of Apalache and the evaluator in place and nothing listening on
port 8822. `set -eu` stops it at the first command that fails, the trap puts back
whatever a stopped run had moved, and a signal leaves through the same trap. A backup is
copied under another name and renamed, so a copy cut short is never taken for one. The
script refuses to start while `ai_tmp/qm01/v/` still holds a backup: that is an
interrupted run's, and it is put back by hand, not by a rerun.

```sh
set -eu
# QM01's Verification, corrected under the maintainer's rulings at step 4 (Q6
# to Q8): case-specific refusal reasons, `absent` requiring grep's status 1,
# Apalache launch and download markers, the lsof -t PID control, curl exit
# codes checked and their positive controls outside the denial, and every
# verify under a denial that allows loopback only. Run with sh from the root.
git rev-parse HEAD

v=ai_tmp/qm01/v
mkdir -p "$v"
pin=tools/quint/evaluator.txt
dir=.quint/rust-evaluator-v0.8.0
stamp=$dir/.pachinko-stamp
smoke=ai_tmp/qm01/smoke.qnt
why='install-quint-evaluator --check: refused:'
src=tools/quint/node_modules/@informalsystems/quint/dist/src
py=$PWD/.pixi/envs/default/bin/python

# A leftover is an interrupted run's backup. Put it back by hand before a rerun.
for f in pin.saved stamp.saved binary.saved; do
    test ! -e "$v/$f"
done

restore() {
    rm -f "${v:?}/pin.tmp" "${v:?}/binary.tmp"
    [ ! -f "$v/pin.saved" ] || mv "$v/pin.saved" "$pin"
    [ ! -f "$v/stamp.saved" ] || mv "$v/stamp.saved" "$stamp"
    [ ! -f "$v/binary.saved" ] || mv "$v/binary.saved" "$dir/quint_evaluator"
}
trap restore EXIT
trap 'exit 1' HUP INT TERM

guard() {
    env QUINT_HOME=.quint PATH="$PWD/.pixi/envs/default/bin:$PATH" \
        sh scripts/install-quint-evaluator.sh --check -- "$@"
}

# absent <fixed string> <file>: grep must exit exactly 1.
absent() {
    status=0
    grep -F -- "$1" "$2" || status=$?
    test "$status" -eq 1
}

# refused <name> <case's own reason> <command>: exit 1 exactly, the guard's or
# the gate's own message with this case's reason, no download.
refused() {
    name=$1
    want=$2
    shift 2
    status=0
    "$@" >| "$v/$name.log" 2>&1 < /dev/null || status=$?
    echo "refused $name: exit $status"
    test "$status" -eq 1
    grep -F -- "$want" "$v/$name.log"
    absent 'Fetching Rust evaluator' "$v/$name.log"
    absent 'Downloading Apalache distribution' "$v/$name.log"
}

[ "$(uname -s)" = Darwin ] || exit 1
deny() { sandbox-exec -p '(version 1)(allow default)(deny network*)' "$@"; }
loop='(version 1)(allow default)(deny network*)(allow network-bind network-inbound (local ip "localhost:*"))(allow network-outbound (remote ip "localhost:*"))'
deny_loop() { sandbox-exec -p "$loop" "$@"; }

# curl_denied <denier>: launches, finds curl, and both controls fail with a
# network error, not a missing or refused command.
curl_denied() {
    "$1" true
    "$1" sh -c 'command -v curl' > /dev/null
    status=0
    "$1" curl -fsS --max-time 10 https://static.rust-lang.org -o /dev/null || status=$?
    echo "$1 control 1 curl exit $status"
    case $status in 0 | 2 | 126 | 127) exit 1 ;; esac
    status=0
    "$1" curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1 -o /dev/null || status=$?
    echo "$1 control 2 curl exit $status"
    case $status in 0 | 2 | 126 | 127) exit 1 ;; esac
}

# The markers the silent probes look for, as the CLI's source spells them.
grep -F 'Fetching Rust evaluator' "$src/rust/binaryManager.js"
grep -F 'Starting Rust evaluator with command:' "$src/rust/commandWrapper.js"
grep -F 'Downloading Apalache distribution' "$src/apalache.js"
grep -F 'No running Apalache server found, launching...' "$src/apalache.js"

# Gate 2: green, then red on a stale quint=.
just lock-check
cp "$pin" "$v/pin.tmp"
mv "$v/pin.tmp" "$v/pin.saved"
sed 's/^quint=.*/quint=0.33.0/' "$v/pin.saved" >| "$pin"
refused lock "check-quint-evaluator-pin: $pin says quint=0.33.0, but" just lock-check
mv "$v/pin.saved" "$pin"
git diff --exit-code "$pin"

# Read-only first: the run stops here unless the stamped install is in place,
# so the installer below should only skip. It runs under the denial, so that an
# installer that disagrees with the guard fails and downloads nothing.
guard run
deny just install-quint-evaluator >| "$v/install.log" 2>&1
grep -F 'nothing to do.' "$v/install.log"

# The guard without the stamp. The just quint refusals run under the full
# denial, so a guard that regressed could reach neither the network nor 8822.
mv "$stamp" "$v/stamp.saved"
refused nostamp-run 'no stamp at' deny just quint run --max-steps=3 "$smoke"
refused nostamp-test 'no stamp at' deny just quint test "$smoke"
refused nostamp-verify 'no stamp at' deny just quint verify --invariant=inRange --max-steps=5 "$smoke"
refused nostamp-typescript 'does not read --backend' deny just quint run --backend typescript --max-steps=3 "$smoke"
refused nostamp-default 'there are no arguments' guard
refused nostamp-repl "the first argument is 'repl'" guard repl
refused nostamp-option "the first argument '--verbosity=4'" guard --verbosity=4
just quint typecheck "$smoke"
just quint compile --target json "$smoke" > /dev/null
mv "$v/stamp.saved" "$stamp"

# The guard with a changed binary: a stand-in that would leave a sentinel if it ran.
cp -p "$dir/quint_evaluator" "$v/binary.tmp"
mv "$v/binary.tmp" "$v/binary.saved"
printf '#!/bin/sh\n: >| "%s"\nexit 3\n' "$PWD/$v/sentinel-ran" >| "$dir/quint_evaluator"
status=0
"$dir/quint_evaluator" || status=$?
test "$status" -eq 3
test -f "$v/sentinel-ran"
rm -f "${v:?}/sentinel-ran"
refused changed-run 'tree digest mismatch' deny just quint run --max-steps=3 "$smoke"
test ! -e "$v/sentinel-ran"
mv "$v/binary.saved" "$dir/quint_evaluator"
guard run

# Green, with the network denied. Positive controls outside the denial first.
curl -fsS --max-time 10 https://static.rust-lang.org -o /dev/null
curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1 -o /dev/null
curl_denied deny
deny just quint run --verbosity=4 --max-steps=3 "$smoke" >| "$v/run.log" 2>&1
grep -F 'Starting Rust evaluator with command:' "$v/run.log"
absent 'Fetching Rust evaluator' "$v/run.log"
deny just quint run --backend typescript --verbosity=4 --max-steps=3 "$smoke" >| "$v/typescript.log" 2>&1
grep -F '[ok] No violation found' "$v/typescript.log"
absent 'Starting Rust evaluator with command:' "$v/typescript.log"

# Claim 1, under the loopback-only denial, after its controls. The port probe's
# positive control: lsof -t finds the listener this script started, by PID.
curl_denied deny_loop
deny_loop "$py" -I -B -c '
import socket, threading
srv = socket.socket(); srv.bind(("127.0.0.1", 0)); srv.listen(); port = srv.getsockname()[1]
def serve():
    c, _ = srv.accept(); c.sendall(b"ok"); c.close()
threading.Thread(target=serve, daemon=True).start()
cl = socket.create_connection(("127.0.0.1", port), timeout=5); assert cl.recv(2) == b"ok"
'
port_free() {
    status=0
    lsof -nP -iTCP:8822 -sTCP:LISTEN || status=$?
    test "$status" -eq 1
}
"$py" -I -B -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(5)' &
listener=$!
sleep 1
test "$(lsof -nP -t -iTCP:8822 -sTCP:LISTEN)" = "$listener"
kill "$listener"
wait "$listener" || true
sleep 1

# A verify log: Quint started its own Apalache, and nothing was downloaded.
own_server() {
    grep -F 'No running Apalache server found, launching...' "$1"
    grep -F 'Started Apalache server on pid=' "$1"
    absent 'Connecting with existing Apalache server' "$1"
    absent 'Downloading Apalache distribution' "$1"
    absent 'Apalache server exited with code' "$1"
    absent 'Fetching Rust evaluator' "$1"
}

port_free
deny_loop just quint verify --verbosity=4 --invariant=inRange --max-steps=5 "$smoke" >| "$v/verify-ok.log" 2>&1
grep -F '[ok] No violation found' "$v/verify-ok.log"
absent 'Starting Rust evaluator with command:' "$v/verify-ok.log"
own_server "$v/verify-ok.log"
port_free
# The recipe unsets the proxy variables: a proxy that is not there changes nothing.
env -u no_grpc_proxy -u no_proxy \
    grpc_proxy=http://127.0.0.1:9 https_proxy=http://127.0.0.1:9 http_proxy=http://127.0.0.1:9 \
    sandbox-exec -p "$loop" just quint verify --verbosity=4 --invariant=inRange --max-steps=5 "$smoke" >| "$v/verify-proxy.log" 2>&1
grep -F '[ok] No violation found' "$v/verify-proxy.log"
own_server "$v/verify-proxy.log"
port_free
status=0
deny_loop just quint verify --verbosity=4 --invariant=belowTwo --max-steps=5 "$smoke" >| "$v/verify-violation.log" 2>&1 || status=$?
test "$status" -eq 1
grep -F '[violation] Found an issue' "$v/verify-violation.log"
grep -F 'Starting Rust evaluator with command: evaluate-at-state-from-stdin' "$v/verify-violation.log"
grep -Fx "  $(printf '\342\235\214') belowTwo" "$v/verify-violation.log"
own_server "$v/verify-violation.log"
port_free
status=0
deny_loop just quint verify --verbosity=4 --observation=reachedTwo --max-steps=5 "$smoke" >| "$v/verify-observed.log" 2>&1 || status=$?
echo "verify --observation exit $status"
test "$status" -eq 0
grep -F 'Starting Rust evaluator with command: evaluate-at-state-from-stdin' "$v/verify-observed.log"
grep -F '[observed] Observed reachedTwo, which stopped the run' "$v/verify-observed.log"
own_server "$v/verify-observed.log"
port_free
guard run

just check-docs
just check-agents
just check
status=$(git status --short)
test -z "$status"
echo "VERIFICATION PASSED"
```

The `refused` lines are the red tests: the gate on a stale `quint=`, the guard without
the stamp for `run`, `test`, `verify`, a TypeScript `run` and the three REPL forms, and
the guard on a changed binary. Each must exit exactly 1 and print its own diagnostic, so
a failure for another reason does not pass. The green tests are `just lock-check`, the
installer's skip, `typecheck` and `compile` without the stamp, the two runs under the
denial, the first `verify` and the `verify` with a dead proxy named. That last line
unsets `no_grpc_proxy` and `no_proxy`, which would otherwise let it pass without the
recipe's help. It is written for the recommended answer on the proxy variables, and
its red half is step 7's, run before the recipe changed; under the alternative it is
dropped, and every `verify` line here gains
`env -u grpc_proxy -u https_proxy -u http_proxy`. The
`nostamp-typescript` line is written for the recommended answer on `--backend`; under
that alternative it becomes a green run under the denial.

Each probe that passes by printing nothing follows its positive control. The two `grep`
lines over the CLI's source show that the markers are the strings the CLI prints. In
`run.log` the start line is found before the fetch line is looked for. `typescript.log`
must hold its result line before the start line is looked for, and `run.log`, from the
same module at the same verbosity, has shown that the start line prints. In the three
`verify` logs the violating and the observed run show the start line that the passing run
must lack. The start line alone shows that the CLI was about to spawn the child, so each
of those two runs must also show what the evaluator's answer prints: the cross-mark line
that names `belowTwo`, and exit status 0 with the `[observed]` line. A child that failed
to launch, or failed on the trace, prints neither. The stand-in is run once by hand, and
leaves its sentinel, before the guarded run that must leave none. `deny true` shows the
sandbox launches, so the two `curl` failures are the denial's. `lsof` finds a listener the
script started before `port_free`, which runs before each `verify` and after the last, is
trusted to find none. The result lines `[ok] No violation found` and
`[violation] Found an issue` were read in `cliCommands.js` line 409 and `cliReporting.js`
lines 234 and 246, the `[observed]` line at line 115 and the cross-mark line at line 171,
none of them run; a control that shows a different spelling is corrected here and
recorded under Departures. An exit status that differs is not a spelling: it is claim 1's
answer, and a **stop**. Steps 4 to 6 hold the other mutations, whose output is quoted
there.

## Hand-back notes

### 1. Base and branch

Base `8eeca83e8b64d9736c40e65866904fdef1af4cfe` on `mk-quint-spec-tickets`, the branch
inside PR #55. The work ran there, in this worktree, not on `ticket/qm01-evaluator-pin`,
as the maintainer ruled before the work (Departures, item 1). It landed as
`5140f88852b5f0872f57a117843fdd5a1ef1640b`.

After that commit, `git fetch origin` left `origin/mk-quint-spec-tickets` at `8eeca83`,
and `origin/quint-spec` at `cf1f3f3` was already an ancestor of `HEAD`. Nothing was
merged or rebased, so no file under Files touched changed after checkpoint 2 that way.

### 2. Baseline

At `8eeca83`: `just initialize` exited 0 (`ai_tmp/qm01/initialize.log`). `just check` on
the untouched worktree then ended (`ai_tmp/qm01/baseline-check.log`):

```text
All checks passed and the worktree is unchanged.
```

`git status --short` printed nothing. `ls .quint` listed only `apalache-dist-0.62.4`, so
there was no evaluator directory of Quint's own. Step 5 seeded one from
`ai_tmp/qm01/fixture/`.

### 3. Claims

Steps 2 and 4 to 7 ran at `8eeca83` with the work uncommitted. Each case's raw log is
under `ai_tmp/qm01/`.

1. **Holds.** Step 7 (`step7-green.out`, excerpt), under the loopback-only denial:

   ```text
   commit 8eeca83 (with uncommitted changes)
   -- verify inRange: exit 0
   [ok] No violation found (3051ms).
   -- verify belowTwo: exit 1
   [violation] Found an issue (3396ms).
   Starting Rust evaluator with command: evaluate-at-state-from-stdin
     ❌ belowTwo
   -- verify observation: exit 0
   Starting Rust evaluator with command: evaluate-at-state-from-stdin
   Starting Rust evaluator with command: evaluate-at-state-from-stdin
   [observed] Observed reachedTwo, which stopped the run (3158ms).
   ```

   The `inRange` log holds no start line. The script asserts that (`step7.sh` line 168,
   `grep` status 1) and does not print it. Each log also held
   `No running Apalache server found, launching...` and `Started Apalache server on pid=`.
   The same three runs passed again at `5140f88` (section 10).
2. **Holds.** First half, step 7 under §11's full denial: `just quint run --verbosity=4`
   exited 0 and logged `Starting Rust evaluator with command: simulate-from-stdin`. The
   absence of `Fetching Rust evaluator` is asserted at `step7.sh` line 97. Second half,
   step 5 case 15, the one unguarded run, under the full denial after `true` exited 0 and
   the two `curl` controls exited 6 and 7 inside it (and 0 outside it). The stand-in
   exited 3 when run by hand. `just quint run --max-steps=3 ai_tmp/qm01/smoke.qnt` exited
   1 with `error: [QNT516] Rust evaluator exited with code 3`. The sentinel file was there
   and the log held no fetch line, so the CLI ran what it found. These values come from
   the first step-5 run at `8eeca83` with the work uncommitted. `step5.out` and
   `step5/unguarded-*.log` on disk come from a rerun after step 6 had added the guard, so
   case 15 there is the guard's refusal (`step 5 rc=1`). That rerun is not evidence for
   this claim (Departures, item 11).
3. **No checksum asset.** The release `evaluator/v0.8.0` lists no checksum file. Every
   asset carries a GitHub-computed `digest`, served by the same API and host as the
   download, so it is not an independent checksum. For both pinned archives, the digest
   agreed with `sha256sum` and with `shasum -a 256` of the download (step 2, step 4's
   re-hash). Each pinned hash therefore rests on one download over TLS. The pages say so.
4. **Holds.** In the first step-5 run, case 15, `just quint run --backend typescript`
   exited 0 under the full denial with no sentinel file. In step 7, under the full
   denial, the TypeScript run exited 0 with `[ok] No violation found (62ms at 161290
   traces/second).` The script asserts it held no start line. The guard fails closed:
   section 6 shows every `--backend` spelling refused without the stamp.
5. **A, holds.** The release (`ai_tmp/qm01/release.json`, read without a credential, not
   draft, not prerelease) has five assets: `quint_evaluator-aarch64-apple-darwin.tar.gz`,
   `-aarch64-unknown-linux-gnu.tar.gz`, `-x86_64-apple-darwin.tar.gz`,
   `-x86_64-pc-windows-msvc.zip` and `-x86_64-unknown-linux-gnu.tar.gz`. These include
   the two that `binaryManager.js` lines 132 and 143 name.
6. **B, holds.** `tar -tzf` and `tar -tvzf` (bsdtar 3.5.3, the four `TAR_*`/`TAPE`
   variables unset) list each archive as exactly one member, `quint_evaluator`, a regular
   file with mode `-rwxr-xr-x`: 2200712 bytes for Linux and 2511024 for Darwin. Neither
   has a `./` prefix or a directory member. The member rule is fixed from this (section
   9, item 4).
7. **C, holds.** Each `browser_download_url` is
   `https://github.com/quint-co/quint/releases/download/evaluator/v0.8.0/<asset>`. The
   downloads hash to the assets' digests:
   `774020b648a887f511d040fd564a8223c62a996b80f8dc50ea24d8630235be51` for `linux-64` and
   `cc016a73031f8717f7c7b9f2a54eb21b15d58b2a1f1ee7f1750a225e00a76672` for `osx-arm64`.
8. **D, holds.** Step 7: the Rust `run` exited 0 under
   `sandbox-exec -p '(version 1)(allow default)(deny network*)'`, with no loopback
   exception. Inside it, `true` exited 0 and the controls exited 6 and 7. The REPL,
   piped `1 + 1` under the same denial, exited 0, printed `>>> 2` and showed the banner
   `Quint REPL 0.34.0 (using the Rust backend)`.
9. **E, holds on this machine.** `uname -s` gave `Darwin`, `uname -m` gave `arm64`, and
   pixi's `node` gave `darwin arm64`. Only one machine was checked. Under ruling Q4 the
   guard reads pixi's `node`, not `uname` (section 4).
10. **F, holds.** The start line appeared at `--verbosity=4` in every log that started
    the evaluator, and it is the positive control in each silent probe's own log. The
    fetch line was matched against its source line, `binaryManager.js` line 57,
    `Fetching Rust evaluator ${version}...`, which `verify.sh` prints first (section 10).
    No run downloaded through Quint, so the fetch line was never seen printed.
11. **G, holds.** Red, step 7 (`step7-red.out`), with the recipe as step 6 left it and
    nothing on `127.0.0.1:9` or 8822. The run used the loopback-only denial (section 9,
    item 10):

    ```text
    commit 8eeca83 (with uncommitted changes)
    -- proxy red: exit 1 after 60s
    E Failed to connect to proxy 127.0.0.1:9 with error connect ECONNREFUSED 127.0.0.1:9
    error: Timed out waiting for Apalache server to become ready after 60 seconds. Channel state: 3
    ```

    Green, after the six `-u` flags (`step7-green.out`): `proxy green (grpc_proxy): exit
    0` and `proxy green (all six): exit 0`, each with `[ok] No violation found`.

### 4. Design rulings

The maintainer ruled at step 4 (2026-11-25) on the plan's twelve questions
(`ai_tmp/qm01/plan.md`, "Rulings at step 4"):

- Q1, the pin layout and stamp: (a), as recommended.
- Q2, the guard's table: (a), as recommended. `--backend` is never read, so
  `run --backend typescript` is refused without the install.
- Q3, the proxy variables: (a), six `-u` flags on the recipe.
- Q4, the platform test: **(b), pixi's `node`** (`process.platform`, `process.arch`),
  not `uname`. This differs from the recommendation. The installer asks
  `.pixi/envs/default/bin/node` with `NODE_OPTIONS` unset. It refuses unless that is the
  first `node` on `PATH`. The mapping is `linux x64` to `linux-64` and `darwin arm64` to
  `osx-arm64`. `rg -n -i 'uname|rosetta|process\.arch' .scratch/quint-model/` outside
  QM01 finds only `QM10-rust-replay.md` line 444, which quotes `uname -sm` for where
  traces were made and does not depend on the platform test. So no sentence in the README
  or in QM02 to QM13 was edited.
- Q5, `NODE_OPTIONS` unset in the recipe: (a). Q6, the corrections to Verification: (a),
  all of them (section 9, item 6). Q7, positive `curl` controls outside the denial: (a),
  authorised. Q8, a loopback-only denial for every `verify`: (a).
- Q9, the 8822 window: (a), an Open point plus the absent marker
  `Apalache server exited with code`. Q10, edit `docs/how-to/develop-locally.md:152`:
  (a). Q11, decision 0021 line 29: (a), left as it is.
- Q12, the six stricter checks, `--check-archive` and the one-member rule: keep all.

After checkpoint 2 (2026-11-25):

- Keep the CLI's line check, with a by-hand read of the lookup on each bump. The round-2
  high stays open ("Where a later CLI looks").
- A mount under the old directory is recorded in Open points only. Neither installer
  changes.

### 5. What landed

`5140f88`, "Pin the Quint CLI's Rust evaluator by checksum (QM01)", `git show --stat`:

```text
 .dockerignore                                    |   5 +-
 .gitignore                                       |   9 +-
 .scratch/quint-model/QM01-evaluator-pin.md       | 205 ++++++---
 AGENTS.md                                        |   2 +-
 Justfile                                         |  79 +++-
 docs/decisions/0021-project-managed-quint-cli.md |  71 ++-
 docs/explanation/security-model.md               |  30 +-
 docs/how-to/develop-locally.md                   |  21 +-
 docs/how-to/maintain-dependencies.md             |  87 +++-
 docs/operations/maintenance.md                   |   3 +-
 docs/project/repository-map.md                   |   4 +-
 docs/reference/commands.md                       |  27 +-
 docs/reference/configuration.md                  |   6 +-
 docs/reference/quality-gates.md                  |   6 +-
 docs/reference/testing.md                        |   2 +-
 docs/tutorials/first-change.md                   |   2 +-
 renovate.json                                    |   2 +-
 scripts/check-quint-evaluator-pin.sh             | 142 ++++++
 scripts/initialize.sh                            |  13 +-
 scripts/install-quint-evaluator.sh               | 549 +++++++++++++++++++++++
 tools/quint/evaluator.txt                        |  13 +
 21 files changed, 1125 insertions(+), 153 deletions(-)
```

- `tools/quint/evaluator.txt` pins `quint=0.34.0`, `evaluator=v0.8.0`, and the two hashes
  from claim C.
- `scripts/install-quint-evaluator.sh` has an install mode, `--check` (the guard) and
  `--check-archive`.
- `scripts/check-quint-evaluator-pin.sh` is in `just lock-check`.
- The `quint` recipe gains a second guard line and unsets `NODE_OPTIONS` and the six
  proxy variables.
- `scripts/initialize.sh` runs `just install-apalache` at line 48 and
  `just install-quint-evaluator` at line 54.
- `AGENTS.md` line 37 lists `just install-quint-evaluator` among the recipes that reach
  the network.
- Decision 0021 and the handbook pages state the pin and the gaps under Goal.

Every path is in Files touched. The ticket's own diff is the corrected Verification and
the Open points.

### 6. The gate, the installer and the guard, red and green

Every control ran on `osx-arm64` only (macOS, Darwin arm64): the gate, the installer,
the guard, both denials and every `verify`. The `linux-64` hash was pinned from a
download that was hashed and listed but never unpacked or run (Open points, "The other
platform"). Linux's `unshare -rn` and GNU `tar` and `find` were never run.

**Step 4, the gate** (`ai_tmp/qm01/step4.out`, `step 4 rc=0`):

```text
commit 8eeca83 (with uncommitted changes)
green green: exit 0
green green-absent: exit 0
red quint-0.33.0: exit 1
check-quint-evaluator-pin: tools/quint/evaluator.txt says quint=0.33.0, but tools/quint/package.json pins '0.34.0' and tools/quint/package-lock.json locks '0.34.0'. After a Quint bump, read the new CLI's QUINT_EVALUATOR_VERSION, move evaluator= and every sha256.<platform>= if it moved, then set quint= (docs/how-to/maintain-dependencies.md, "Moving the evaluator pin")
red no-linux: exit 1
check-quint-evaluator-pin: tools/quint/evaluator.txt must have exactly one sha256.linux-64= line, for the pixi platform linux-64; it has 0
red 63-digits: exit 1
check-quint-evaluator-pin: sha256.osx-arm64= in tools/quint/evaluator.txt is not 64 lowercase hex digits: c016a73031f8717f7c7b9f2a54eb21b15d58b2a1f1ee7f1750a225e00a76672
red osx-64: exit 1
check-quint-evaluator-pin: tools/quint/evaluator.txt has sha256.osx-64=, but osx-64 is not a platform under [tool.pixi.workspace] in pyproject.toml
red no-v: exit 1
check-quint-evaluator-pin: evaluator=0.8.0 in tools/quint/evaluator.txt is not v followed by x.y.z, as the CLI's QUINT_EVALUATOR_VERSION spells it
red apalache-key: exit 1
check-quint-evaluator-pin: tools/quint/evaluator.txt has a line that is not quint=, evaluator=, sha256.<platform>=, a comment or blank: apalache=0.62.4
cmp exit 0
dash green exit 0
dash red exit 1
```

`green-absent` ran with `tools/quint/node_modules` and `.quint` moved into `ai_tmp/qm01/`.
The `dash` red printed the same `quint=0.33.0` message, and `cmp` exited 0 after it.

**Step 5, the installer** (`ai_tmp/qm01/step5.out`, cases 1 to 14, from the rerun with
the same results as the first run):

- Cases 1 to 6 each exited 0. Each install among them (all but case 2) ended with
  `installed the Rust evaluator v0.8.0 (osx-arm64, sha256 cc016a73…6672)`.
  - Case 1, Quint's own directory (the fixture): `no stamp at
    .quint/rust-evaluator-v0.8.0/.pachinko-stamp`, replaced.
  - Case 2: a second run skipped with `the Rust evaluator v0.8.0 (osx-arm64) is installed
    and matches its stamp; nothing to do.` The same line printed under `dash`.
  - Case 3, one byte appended, and case 4, the asset's name beside the executable: each
    `tree digest mismatch`, replaced.
  - Case 5, removed and repopulated: `no stamp`, replaced.
  - Case 6, `chmod a-x`: `executable not a regular executable file`, replaced.
- Cases 7 to 11 each exited 1. Cases 8 to 10 left `.quint` unchanged by snapshot, and
  case 7 left its copy unchanged.
  - Case 7, a symlink at the directory: `… is a symlink; it must be a directory inside
    this worktree. Remove the symlink itself (not its target), then run just
    install-quint-evaluator`. The copy was unchanged.
  - Case 8, `evaluator=v0.7.0`: `the installed Quint CLI 0.34.0 runs the evaluator
    'v0.8.0' (QUINT_EVALUATOR_VERSION), but tools/quint/evaluator.txt pins
    evaluator=v0.7.0`.
  - Case 9, `package.json` at `0.33.0`: `the installed Quint CLI is 0.33.0, not
    quint=0.34.0 from tools/quint/evaluator.txt; run just install-quint`. `cmp` exited 0
    after the restore.
  - Case 10: `QUINT_HOME must be .quint, … (it is 'ai_tmp/qm01/elsewhere')`.
  - Case 11, the wrong hash: `checksum mismatch for …: tools/quint/evaluator.txt pins
    sha256.osx-arm64=0c016a73…, the download has cc016a73…; nothing was installed`.
    `.quint` then held only `apalache-dist-0.62.4`.
- Case 12, the member check through `--check-archive` on crafted archives, none
  unpacked. `good` and the real archive exited 0. These exited 1:

  ```text
  archive check: unsafe component in '../quint_evaluator'
  archive check: absolute name '/tmp/quint_evaluator'
  archive check: not a regular file: 'quint_evaluator' (symbolic link)
  archive check: not a regular file: 'quint_evaluator' (hard link)
  archive check: unexpected member 'other'
  archive check: duplicate member quint_evaluator
  archive check: not a regular file: 'quint_evaluator' (directory)
  ```

  - Case 12b, install mode, offline, with the symlink archive served by a stand-in
    `curl`: refused by the member check, and `tar never ran`.
  - Cases 12c and 12d, a `tar` that unpacks a hard link or a symlink where the archive
    held a regular file: `the unpacked quint_evaluator is a hard link to another file;
    nothing was installed`, and `entry check: quint_evaluator is not a regular file`.
    Each exited 1.
- Case 13, a stand-in `node` first on `PATH`: exit 1, `the first node on PATH is
  …/ai_tmp/qm01/fakenode/node, not pixi's …/.pixi/envs/default/bin/node`. The
  other-platform branch was not run (section 9, item 9).
- Case 14: with the pin restored (`cmp` exited 0), an install and then a skip, each exit
  0.
- Case 15: section 3, claims 2 and 4.

**Step 6, the guard** (`ai_tmp/qm01/step6.out`, `step 6 rc=0`). Every `just quint`
command ran under the full denial after `true` and the two `curl` controls, which exited
6 and 7. Each refusal exited 1 and named `just install-quint-evaluator`. The script
asserted that each left an unchanged snapshot of `.quint` and no fetch line. With the
stamp moved aside, `run`, `test`, `verify`, the guard with no arguments, with `repl`,
with `--verbosity=4`, with `nonsense` and with `verify --help` were each refused with
`(no stamp at .quint/rust-evaluator-v0.8.0/.pachinko-stamp)`. For example:

```text
commit 8eeca83 (with uncommitted changes)
-- refused nostamp-run: exit 1
install-quint-evaluator --check: refused: the Rust evaluator v0.8.0 is not installed as tools/quint/evaluator.txt pins it (no stamp at .quint/rust-evaluator-v0.8.0/.pachinko-stamp); run just install-quint-evaluator. Checked because the first argument is 'run', which runs the evaluator; the guard does not read --backend, so --backend typescript is refused too
```

`typecheck`, `parse`, `compile --target json` and `--version` passed without the stamp.
All eight `--backend` vectors were refused, as ruling Q2 says:

- `--backend=typescript`, `--backend typescript` and `--backend="typescript"`
- `--no-backend` and `--BACKEND=typescript`
- the option given twice
- `run -- --backend=typescript …`
- `run … <file> -- --backend=typescript`

The stamp was restored (`cmp` exited 0). Then:

- The stand-in was refused with `tree digest mismatch`, and the script asserted that no
  sentinel appeared.
- An appended byte was refused with `tree digest mismatch`, and `chmod a-x` with
  `executable not a regular executable file`.
- A symlinked directory was refused.
- An empty directory inside it was refused with `unexpected entries`.
- A symlink at `dist/src/config.js` and at `@informalsystems` were each refused with
  `is a symlink`.
- An extensionless `config` or `rust/binaryManager` beside its `.js` was refused with
  `node's require() would load it`.
- An extra line naming the constant was refused with `names QUINT_EVALUATOR_VERSION on
  lines other than the three this check knows`.
- A stamp with a trailing blank line was refused with `stamp malformed`.
- `evaluator=v0.7.0` was refused, naming both versions.
- `QUINT_HOME=ai_tmp/qm01/elsewhere` was refused.

With everything restored, `just quint run` exited 0 (`passed green-run`), and the guard
under `dash` exited 0.

**Step 7** (`step7-full.out`, `step 7 full rc=0`). Under the full denial, the TypeScript
run is in section 3 (claim 4). The CLI's own reading of each `--backend` spelling, with
the stamp in place:

- `--backend=typescript`, `--backend typescript` and `--backend="typescript"`: exit 0, no
  evaluator start.
- `--no-backend`, `--BACKEND=typescript` and the option given twice: exit 1 from the CLI,
  after the guard had passed, with no evaluator start.
- `run … <file> -- --backend=typescript`: exit 0, and the evaluator started.

The REPL, the proxy runs and claim 1 are in section 3.

### 7. Checkpoint 1 record: the plan

**Harness and model.** Codex CLI v0.172.0, `codex exec -m gpt-6.2-sol -s read-only`, run
in the form the review deltas give. It read `ai_tmp/qm01/plan.md` and the drafts at
`8eeca83`, before any tracked file changed. The lenses were conformance and breakage,
and there were two rounds. Fresh Claude subagents triaged and applied the fixes, one
writer at a time. The records are under `ai_tmp/review/QM01-1/` (`findings-*.md`,
`verdicts.md`).

**Round 1.** 19 findings, 15 distinct (B01 = C02, B03 = C01's `sed` part, B08 = C06's
digest part, B09 = C05 + C06). Three were high (C01, C02, B01). Over all 19: 14 valid, 5
valid in part, 0 invalid.

- C01 (high), valid in part: the symlink list missed `tools`, `tools/quint` and
  `config.js`, and one `sed` line could not see a later write of the constant. Added to
  `check_cli`, and every line naming the constant must equal one of 0.34.0's three
  (Departure 6 of the plan).
- C02/B01 (high), valid: `lsof` accepted any listener, and no log was checked for
  Quint's own launch. Launch markers are now required, the existing-server marker must be
  absent, and the `lsof -t` PID must equal the listener's.
- C03, valid: the Apalache download marker was not searched for, and the red proxy run
  accepted any failure. The marker must be absent, and the red run needs a gRPC
  connection error.
- C04, valid: the branch. Closed by the maintainer's ruling.
- C05, valid: refusals matched only the shared prefix, and `grep` status 2 counted as
  absent. Each refusal now matches its own reason, through an `absent` helper.
- C06/B08, valid: the `ls -la` snapshot was shallow, and AP02's digest skips
  directories. There is now a recursive snapshot, and the skip test checks the entries.
- C07, valid in part: steps 8 to 10 were not sequenced. Added to the plan.
- C08, valid in part: page lines were missing from the plan's map. Added. Line 152 and
  decision 0021 line 29 became Q10 and Q11.
- C09 and C10, valid: the added controls were not listed, and the `uname` tradeoff was
  misstated. Both are corrected in the plan.
- B02, valid: `NODE_OPTIONS` could set `QUINT_HOME` after the guard. Became Q5.
- B04, valid in part: the full prefixes in the plan.
- B05, valid: success relied on the trap. The installer now removes `$tmp` itself.
- B06, valid in part: install mode's member check had no red control. Case 12b added.
- B07, valid: any non-zero `curl` status passed. A status of 0, 2, 126 or 127 now fails.

**Round 2** (the last). 20 findings, 13 distinct. Three were high (R2-C01 = R2-B02,
R2-C02 = R2-B04, and R2-B01 in part). Over all 20: 16 valid, 4 valid in part, 0 invalid.

- R2-C01/R2-B02 (high), valid: Quint retries 8822 without tying the answer to its child.
  No fix fits within shared rule 10. Became Q9, and the absent marker
  `Apalache server exited with code` was added.
- R2-C02/R2-B04 (high), valid: step 6 ran `verify` without a denial. Every `just quint`
  in steps 5 and 6 now runs under the full denial. Became Q6 to Q8 for step 7 and
  Verification.
- R2-B01 (high), valid in part: an extensionless `config` or `rust/binaryManager` would
  be loaded instead of the checked file. These are refused (Departure 8).
- R2-C03 to R2-C09 and R2-B03 to R2-B11 were each valid or valid in part:
  - the step scripts record `rc` and restore through an EXIT trap;
  - pixi's `python` is used by path;
  - the extra-key and npm comparisons run in Python;
  - case 15 and the eighth vector were added;
  - `find -links 1` and cases 12c and 12d were added;
  - the stamp is compared with `cmp`;
  - the symlink copy is checked by a content hash;
  - the cleanup wording was corrected.
- R2-B11, the mount under the old directory, was left open.

**Applied** to `ai_tmp/qm01/` only. No tracked file changed before the step-4 ruling.

**Rulings.** Section 4.

**Handed back.** Round 1 handed nothing back. Round 2, both outside Files touched:

- `scripts/install-apalache.sh` `check_cli` (R2-B01): its symlink list does not name
  `dist/src/config.js`, through which Apalache's lookup goes, nor the extensionless
  `dist/src/config`.
- `scripts/check-apalache-pin.sh` lines 63 to 68 (R2-B05): the same `$()` trim of the
  npm values.

**Re-run.** Steps 4 to 7 at `8eeca83` with the work uncommitted (sections 3 and 6).

### 8. Checkpoint 2 record: the diff

**Harness and model.** As at checkpoint 1, over `git diff 8eeca83` plus the uncommitted
changes, with the pages included. There were two rounds. The records are under
`ai_tmp/review/QM01-2/` (`findings-*.md`, `verdicts.md`, `controls.out`,
`controls-2.out`).

**Round 1.** 15 finding ids, 11 verdicts (B1 = C1, B3 = C2, B6 = C7, B7 = C6). One was
high (B1). 8 valid, 3 valid in part, 0 invalid.

- B1/C1 (high), valid: no line of `config.js` or `binaryManager.js` that builds the
  evaluator's path was checked. The new `exact_lines` check compares those lines with
  0.34.0's. The pages state the in-place-edit limit.
- B2, valid: `tarfile` stopped silently at a bad header. The member check now requires
  zeros after the last member.
- B3/C2, valid: `NODE_V8_COVERAGE` reached the guard's `node`. The probe now runs under
  `env -i`.
- B5, valid: a `node` in pixi's `lib/jvm/bin` would run the CLI. It is refused.
- B6/C7, valid: the evaluator bump order and `tar -tvzf` in the how-to. Fixed for the
  evaluator's subsection. The Apalache half was handed back.
- B7/C6, valid: "or an observation" was added on QM01's pages.
- C4, valid: a NUL passed the pin check. The check now accepts printable ASCII and LF
  only.
- C5, valid in part: commands.md wording.
- B4, valid in part (Linux, from source): a mount under the old directory. Not applied;
  left for the maintainer.
- B8, valid: `.scratch/apalache/README.md:53`. Handed back.
- C3, valid in part: the record and Verification. Left to the executing agent.

**Round 2** (the last). 11 finding ids, 7 verdicts. One was high (R2-B1 = C2R2-01).
5 valid, 2 valid in part, 0 invalid apart from R2-B3's path sub-claim.

- R2-B1/C2R2-01 (high), valid: on byte copies, a lookup change outside the compared lines
  passed `check_cli`. Left open by the maintainer's ruling (section 4). The pages were
  corrected under C2R2-05 so they no longer say such a release is refused.
- R2-B4/C2R2-04, valid: `--check-archive` resolved a relative path against the root.
  Fixed.
- R2-B5, valid: a PAX global record. The how-to now runs `--check-archive` on each
  archive.
- R2-B2/C2R2-02: the mount, as in round 1. Recorded in Open points only, by ruling.
- R2-B3/C2R2-03 and R2-B6: Verification, and the Apalache bump order. Handed back.

**Applied** to the scripts, the `Justfile` and the pages, before `5140f88`. Every path is
in Files touched.

**Handed back.**

- The "Moving the Apalache pin" subsection of `docs/how-to/maintain-dependencies.md` runs
  `just lock-check` before the evaluator pin is reviewed (B6, R2-B6). That is the
  Apalache series' lines, and it is in Open points.
- "Or an observation" is missing in `.scratch/quint-model/QM02`, `QM03` (two places),
  `QM05`, `QM13` (two places) and QM01's own design text (B7/C6). These are left as
  written.
- `.scratch/apalache/README.md:53` still reads as current status (B8). It is in Open
  points.
- Verification and the record (C3, R2-B3/C2R2-03) went to the executing agent. This is
  closed: the ticket's Verification block is now the same as `ai_tmp/qm01/verify.sh`
  (`diff` exit 0), with each correction under section 9, item 6. Sections 4, 7, 8 and 9
  are this record.

**Re-run.** At `8eeca83` with the work uncommitted, the review's controls were run on
byte copies in a scratch mirror (`controls.out`, `controls-2.out`). Then:

```text
Validated 49 pages and 50 canonical topics.
Validated AGENTS.md, 2 adapters, and 14 skills.
```

`just lock-check` and `just lint` each exited 0.

**After the last checkpoint.** A `/code-review` at high effort then gave six nits:

- Fixed: comparing pixi's `node` in a symlinked checkout.
- Fixed: a `just check-quint-evaluator-archive` recipe.
- Fixed: a note on mapping a new platform.
- Fixed: an `AGENTS.md` pointer to the gaps.
- Left: the Apalache how-to bump order, which is outside the ticket's lines and in Open
  points.
- Left: the bare `python`, which matches `scripts/check-apalache-pin.sh`.

These four fixes changed files under Files touched after checkpoint 2's last round, and
Codex did not review them. They are in `5140f88` and covered by section 10's run. The
push stop asks whether checkpoint 2 runs over them first.

**After the last checkpoint** (the maintainer asked for it, 2026-11-25). One round of
both lenses, Codex Sol as above, over the changes made after checkpoint 2 (the
`/code-review` fixes, the `scripts/initialize.sh` rewrap, the Verification block and
Open points), at `a8d7be9`; raw files in `ai_tmp/review/QM01-3/`. Nine findings, five
distinct, all valid. Three were in the `node` comparison that let a symlinked checkout
through: a `PATH` entry ending `link/..` collapsed by a logical `cd`, a failed
resolution that kept the unresolved spelling, and a directory name ending in a newline.
Each could approve another `node`. Fixed by comparing the two as files with `test
-ef` and resolving no path; probed in `dash` and `bash`, where the normal case and a
symlinked checkout pass and the `..`, newline and fake-first cases are refused. The
Verification preamble overclaimed for the refused `verify`, and Open points lacked the
8822 window ruled at Q9; both fixed. The fixes are in the commit after `a8d7be9`.

### 9. Departures

1. **Branch and worktree.** The work ran on `mk-quint-spec-tickets` at `8eeca83`, in this
   worktree. There was no `ticket/qm01-evaluator-pin`, no new worktree, and no second
   `just initialize` beyond step 1's. This follows the maintainer's ruling before the
   work.
2. **Reviewer model.** `gpt-6.2-sol`, as the review deltas name it. Codex was on `PATH`,
   so the fallback was not used.
3. **A third installer mode.** `--check-archive <file>` is offline and read-only, so the
   member check can run on its own (step 5, case 12). `just check-quint-evaluator-archive`
   runs it. The design names two modes.
4. **One member.** The member set must be exactly one regular file, `quint_evaluator`.
   Directory members are refused too, because both v0.8.0 archives are that one file
   (claim B).
5. **Stricter checks than the ticket's text** (Q12, kept):
   - the installed entry set in the skip test;
   - the three-line set for the constant;
   - removal of `$tmp` on success;
   - refusal of extensionless shadows of `config` and `rust/binaryManager`;
   - the stamp compared byte for byte;
   - a single link on the unpacked executable;
   - the lines of `config.js` and `binaryManager.js` that build the path (checkpoint 2,
     B1);
   - the `env -i` platform probe;
   - refusal of a `node` in pixi's JVM `bin`;
   - refusal of NUL or CR in the pin.
6. **Verification corrected** under rulings Q6 to Q8:
   - each refusal checks its own reason;
   - an absent marker needs `grep` status 1;
   - each `verify` log must show Quint launching its own Apalache and no download;
   - the port control names its listener's PID;
   - the `curl` controls check exit codes, after positive runs outside the denial;
   - every `verify` runs under a loopback-only denial.

   The script runs on Darwin only. The Linux `unshare -rn` branch was dropped, because a
   new namespace starts with `lo` down and this was not probed. The loopback profile was
   probed on macOS:

   ```text
   (version 1)(allow default)(deny network*)(allow network-bind network-inbound (local ip "localhost:*"))(allow network-outbound (remote ip "localhost:*"))
   ```

   A first profile with `(allow network* (local ip "localhost:*"))` let `curl` reach
   `1.1.1.1` and was rejected.
7. **`NODE_OPTIONS` is unset** by the `quint` recipe, a seventh `-u` beside the ticket's
   six (Q5).
8. **Platform from pixi's `node`**, not `uname` (Q4, section 4).
9. **Step 5, case 13** became a stand-in `node` first on `PATH`, with the script run
   directly. Pixi's environment has no `uname` for the ticket's case to shadow, and the
   ruling reads `node`. The branch for a platform the pin does not list was not run: it
   needs pixi's `node` replaced.
10. **Step 7's `verify` runs**, the proxy red and green runs included, ran under the
    loopback-only denial of item 6, not "not under a denial" (Q8). Two vectors differ
    from the plan:
    - Step 7 ran seven `--backend` vectors, not eight. `run -- --backend=typescript
      <file>` ran in step 6 only, where it was refused.
    - The vector with the file before `--` ran in both steps.
11. **Reruns in step 5.**
    - The harness's first run of `--check-archive` lacked pixi's `bin` on `PATH` and was
      run again.
    - `step5.out` on disk is a rerun made after step 6 added the guard. Its case 15 is
      the guard's refusal, and the unguarded evidence is the first run's (section 3,
      claim 2).
12. **Extra positive `curl` controls**, outside the denial, in steps 5 and 7 and in
    Verification (Q7, network authorised). Each exited 0.

### 10. Verification

The ticket's Verification block, as `ai_tmp/qm01/verify.sh` (the two are identical), ran
at `5140f88852b5f0872f57a117843fdd5a1ef1640b` after the commit. It exited 0. Under
`set -eu`, it reaches its last line only when every command before it succeeded.
Excerpts from `ai_tmp/qm01/verify.out`:

```text
5140f88852b5f0872f57a117843fdd5a1ef1640b
refused lock: exit 1
check-quint-evaluator-pin: tools/quint/evaluator.txt says quint=0.33.0, but tools/quint/package.json pins '0.34.0' and tools/quint/package-lock.json locks '0.34.0'. After a Quint bump, read the new CLI's QUINT_EVALUATOR_VERSION, move evaluator= and every sha256.<platform>= if it moved, then set quint= (docs/how-to/maintain-dependencies.md, "Moving the evaluator pin")
install-quint-evaluator: the Rust evaluator v0.8.0 (osx-arm64) is installed and matches its stamp; nothing to do.
refused nostamp-run: exit 1
refused nostamp-test: exit 1
refused nostamp-verify: exit 1
refused nostamp-typescript: exit 1
refused nostamp-default: exit 1
refused nostamp-repl: exit 1
refused nostamp-option: exit 1
refused changed-run: exit 1
install-quint-evaluator --check: refused: the Rust evaluator v0.8.0 is not installed as tools/quint/evaluator.txt pins it (tree digest mismatch: the files under .quint/rust-evaluator-v0.8.0 are not the ones .quint/rust-evaluator-v0.8.0/.pachinko-stamp records); run just install-quint-evaluator. Checked because the first argument is 'run', which runs the evaluator; the guard does not read --backend, so --backend typescript is refused too
deny control 1 curl exit 6
deny control 2 curl exit 7
Starting Rust evaluator with command: simulate-from-stdin
[ok] No violation found (60ms at 166667 traces/second).
deny_loop control 1 curl exit 6
deny_loop control 2 curl exit 7
[ok] No violation found (2965ms).
[ok] No violation found (2970ms).
[violation] Found an issue (3102ms).
Starting Rust evaluator with command: evaluate-at-state-from-stdin
  ❌ belowTwo
verify --observation exit 0
[observed] Observed reachedTwo, which stopped the run (3060ms).
Validated 49 pages and 50 canonical topics.
Validated AGENTS.md, 2 adapters, and 14 skills.
All checks passed and the worktree is unchanged.
VERIFICATION PASSED
verify exit 0
```

- Each `refused` line was followed by its own diagnostic (`no stamp at …` for the
  `nostamp-*` cases), omitted here, which the script matched.
- The `curl` controls failed with `Could not resolve host: static.rust-lang.org` (6) and
  `Failed to connect to 1.1.1.1 port 443` (7).
- Each `verify` log held `No running Apalache server found, launching...` and
  `Started Apalache server on pid=`.
- The last lines are `just check`'s.

Every run was on macOS. The record's own `just check` runs after this record is
committed and is not quoted here.

**Rerun after the last fix** (the guard's `node` comparison, section 8). The same
script, `ai_tmp/qm01/verify.sh`, at `fbe65eb9b90276170924f083e74b01147de9a229`, exit 0:
the nine `refused` lines each exited 1, the `curl` controls exited 6 and 7 under both
denials, and it ended:

```text
All checks passed and the worktree is unchanged.
VERIFICATION PASSED
```

`git status --short` printed nothing.

## Open points

- **The other platform.** The executing agent hashes both archives and runs the
  installer, the guard and every control on one platform only. The other platform's
  hash is pinned from a download that was never unpacked or run there. QM03's CI job is
  the first run on `linux-64` if this ticket ran on `osx-arm64`; the maintainer's
  machine is the first the other way round. Settled by whoever first runs
  `just install-quint-evaluator` on the second platform.
- **`--backend typescript`.** The guard refuses a run that needs no evaluator, as the
  maintainer ruled at step 4 (hand-back §4). It costs one install command in a worktree
  that skipped `just initialize`.
- **Checking the constant in CI.** Gate 2 checks that the review was recorded, not that
  `evaluator=` equals the CLI's `QUINT_EVALUATOR_VERSION`: the constant lives only in
  the installed package. QM03's `quint` job installs the CLI and runs the installer,
  which compares them, so from QM03 on a wrong `evaluator=` fails CI there. Until then
  the review is the maintainer's.
- **What the stamp proves.** It detects a local change against the installer's record,
  not a coordinated rewrite of the directory and the stamp together.
- **Which server answered a `verify`.** After its first connection fails, Quint spawns
  Apalache and retries `localhost:8822` without tying the answer to its child, so
  another server that binds the port in that window would answer. The maintainer
  ruled at step 4 (hand-back §4) to accept it as the concurrency that shared rule 10
  already forbids: each `verify` has the port probe before and after, and its log
  must not hold `Apalache server exited with code`. Nothing closes the window.
- **Installing while running.** Shared rule 10 forbids it; nothing enforces it. A
  `just quint run` that has passed the guard while
  `just install-quint-evaluator` moves the directory aside can find no executable, and
  Quint then downloads the evaluator itself. The guarantee holds for non-concurrent use,
  and the next guarded run refuses the unstamped directory. A lock across the guard and
  the whole CLI call would close it, for Apalache as well. The maintainer decides
  whether that is a ticket.
- **Whether upstream publishes a checksum.** It does not (claim 3, step 2): the release
  has no checksum asset. Each asset's GitHub `digest` agreed with two local hashes, but
  it comes from the same host as the download, so each pin rests on one download over
  TLS. Decision 0021's reopen trigger stays.
- **The platform test.** The maintainer ruled at step 4 for pixi's `node`, not `uname`,
  so the platform agrees with the CLI's by construction, Rosetta included, at the cost
  of a `node` start in every guarded run. The branch for a platform the pin does not
  list was not run: it needs pixi's `node` replaced.
- **The proxy variables.** The recipe unsets them for every command, as ruled at step 4,
  so a remote `--server-endpoint` behind a proxy no longer works through it.
- **The loader's variables.** The recipe does not unset `LD_PRELOAD`, `LD_LIBRARY_PATH`
  or the `DYLD_` variables, which reach the evaluator as they reach `node` itself. Not
  probed.
- **Two installers, one pattern.** `scripts/install-quint-evaluator.sh` repeats code
  from `scripts/install-apalache.sh`. A shared file would keep later fixes in one place
  and means editing the Apalache installer, which this ticket does not. The maintainer
  decides whether that is a ticket.
- **Pages outside Files touched.** `.scratch/apalache/README.md`, "Out of scope for
  both", says the evaluator "could be pinned the same way in a later ticket". It is
  another series' record and is left as written.
- **Where a later CLI looks.** The guard compares the lines of `config.js` and
  `binaryManager.js` that name the evaluator's version, asset, release and directory
  with 0.34.0's. A release that changes the lookup through other lines passes: a
  normalisation inside `rustEvaluatorDir`, or a return value of
  `inferAssetAndExecutableNames` that names another executable, leaves every checked
  line as it is (checkpoint 2, round 2, run on byte copies; both reached the stamp
  check). The guard then approves a stamped directory the CLI never reads, and Quint
  downloads the evaluator itself. The bump how-to has the lookup functions read by hand
  on every bump. Asking `node` for the path would close it but reverses "text, not what
  `node` loads"; comparing the whole of both files would close it for those files but
  makes every bump edit the installer. The maintainer ruled on 2026-11-25: keep the line
  check and the by-hand read, and leave this open.
- **An in-place edit of the CLI.** The same text check sees a local edit only on the
  lines it compares; an edit elsewhere in the installed CLI, or a module shadowed
  beside one other than `config.js` and `binaryManager.js`, is not detected. The pages
  say so. Asking `node` would close this too.
- **A mount under the old directory.** The installer moves an evaluator directory it
  does not accept into its temporary directory and removes that with `rm -rf`, on
  success and in the exit trap. On Linux, a bind mount inside the old directory would
  be descended and its files removed. Refusing a directory that holds another device
  changes the ruled "any other directory is replaced", and
  `scripts/install-apalache.sh` has the same pattern. The maintainer ruled on
  2026-11-25: record it here and change neither installer. Found by reading, not run.
- **The Apalache bump order.** "Moving the Apalache pin" in
  `docs/how-to/maintain-dependencies.md` runs `just lock-check` before the evaluator pin
  is reviewed, which then fails on `evaluator.txt`'s `quint=`. Those lines are the
  Apalache series', outside Files touched.
