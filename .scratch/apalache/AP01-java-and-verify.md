---
id: AP01
title: "A pinned Java runtime, and a working just quint verify"
status: done
depends_on: []
parallel_with: []
branch: ticket/ap01-java-and-verify
estimated_size: S
---

# AP01: A pinned Java runtime, and a working `just quint verify`

## Context

The Quint CLI 0.34.0 is pinned in `tools/quint/package.json` and installed by
`just install-quint` into `tools/quint/node_modules/`. The `Justfile` exports
`QUINT_HOME := ".quint"`, so whatever Quint downloads lands in the gitignored `.quint/`.
`just quint <args>` runs the installed CLI by path. Decision 0021,
`docs/decisions/0021-project-managed-quint-cli.md`, records all of this, and under
"Decision" it says: "`quint verify` and TLA+ compilation are left out for now. A gate
that runs either needs a Java runtime, and conda-forge's `openjdk` can be pinned in pixi
when one is wanted." This ticket is that pin.

What the installed package does, read from its source under
`tools/quint/node_modules/@informalsystems/quint/` and not yet run:

- `dist/src/apalache.js` line 128 sets `DEFAULT_APALACHE_VERSION_TAG = '0.62.4'`, and
  `dist/src/cli.js` makes it the default of `--apalache-version`.
- `downloadAndUnpackApalache` (`dist/src/apalache.js` from line 292) fetches
  `https://github.com/apalache-mc/apalache/releases/download/v<version>/apalache.tgz`
  and unpacks it with no checksum. It unpacks into a temporary directory under the
  distribution directory, then renames `apalache/` into place.
- `fetchApalache` (from line 334) skips the download when
  `<QUINT_HOME>/apalache-dist-<version>/apalache/bin/apalache-mc` exists. `config.js`
  line 37, `apalacheDistDir`, builds that path.
- `connect` (from line 490) first tries a server at `--server-endpoint`, whose default
  is `localhost:8822` (`dist/src/cli.js`). It uses any server it finds there. Only when
  none answers does it fetch Apalache and spawn `apalache-mc server --port=8822`, and
  `stopWithQuint` kills that child when Quint exits.

So the first `just quint verify` downloads Apalache into `.quint/apalache-dist-0.62.4/`
and then runs it on a Java runtime. The machine has none here: `/usr/bin/java` on macOS
is a stub that offers to install one, and the pixi environment holds no Java.

Two consequences follow from the source above, and the verification below checks both:

- A server already listening on port 8822 is used, whoever started it. Another worktree's
  `quint verify`, still running, would answer this one's. A run that passes while a
  server is up says nothing about this worktree's Java or its Apalache.
- The `Justfile` exports `PATH`, with `.pixi/envs/default/bin` first, but not
  `JAVA_HOME`, and conda's activation scripts do not run for a recipe. Whether Apalache
  finds pixi's Java depends on how its launcher, `apalache/bin/apalache-mc`, looks for
  one, which nobody here has read yet.

Read these:

- `docs/decisions/0021-project-managed-quint-cli.md`.
- `pyproject.toml`, `[tool.pixi.dependencies]`. Each pin is exact and carries a comment
  saying what it is for. `nodejs`'s comment is the closest precedent.
- `Justfile`, the `QUINT_HOME` comment near the top and the `quint` recipe at the end.
- `tickets/CONVENTIONS.md` §11 (worktrees, green, quoted evidence, checks fail closed,
  network, separately authorised actions), §13 (the network at first run, a gate that
  cannot fire) and §14.

## Goal

`just quint verify` model-checks a Quint module in this worktree. It runs on a Java
runtime pinned in `pyproject.toml` and locked in `pixi.lock`, and it uses Apalache
0.62.4 from `.quint/`. It starts its own server and stops it when it exits, and it
leaves `~/.quint` as it was.

## Claims this ticket checks

Each was read from a package or a channel listing, not run here. A claim that fails is
recorded under Departures, and the step that rests on it stops for the maintainer.

| # | Claim | Check |
| --- | --- | --- |
| 1 | conda-forge publishes `openjdk` 21.0.13.1, an LTS line, for both `linux-64` and `osx-arm64` | `pixi search openjdk` for each platform (network), step 2 |
| 2 | Apalache 0.62.4 runs on Java 21 | its release notes or its launcher, step 2; the verify run, step 5 |
| 3 | `apalache.tgz` unpacks to one top-level `apalache/` holding `bin/apalache-mc` | listing the downloaded archive, step 2 |
| 4 | With pixi's environment first on `PATH`, `apalache-mc` runs pixi's `java`, with or without `JAVA_HOME` | reading the launcher, step 2; the probes of step 5 |
| 5 | Quint's spawned server exits when Quint does | the port and process probes of step 5 |

## Non-goals

- No checksum for Apalache, and no installer. AP02 does both. This ticket lets Quint
  download Apalache itself, as decision 0021 describes.
- No change to the Rust evaluator Quint downloads for `run`, `test` and the REPL.
- No gate and no CI job that runs `quint verify`. There is no `.qnt` module in the
  repository. The scratch module of step 5 lives under `ai_tmp/` and is never committed.
- No change to how Quint picks its server endpoint. The pages document the port.
- No separate pixi environment or feature for Java. Open points holds that alternative.
- No `CHANGELOG.md` entry. It records the crates' changes, and the Quint CLI pin added
  none.

## Adversarial review

This ticket follows `tickets/CONVENTIONS.md` §14: its phases, its lenses, its two rounds
at most, its scope guard and its record. These deltas apply, and AP02 cites them.

- **Reviewer.** Codex, Astra, which is `codex exec -m gpt-6-astra`, is preferred. Fall
  back to a fresh-context `claude-opus-5-5` subagent only if `codex` is not on `PATH`,
  and say so in the hand-back notes. Confirm `codex exec --help` on the day, as §14 asks.
- **Command.** T37 is still todo, so §14's printed command misbehaves in two ways. It
  hangs, because it waits on stdin, and its `-o` overwrites the findings with Codex's
  last message. Use this form instead, run from the worktree:

  ```sh
  codex exec -m gpt-6-astra -s read-only -C "$PWD" \
    -o ai_tmp/review/<id>-<n>/last-<lens>.md "<brief naming findings-<lens>.md>" \
    < /dev/null >| ai_tmp/review/<id>-<n>/log-<lens>.txt 2>&1 &
  ```

  `<id>` is the ticket id, `<n>` the checkpoint's number, and `<lens>` the lens.
  `< /dev/null` closes stdin. `-o` holds only the last message. The log recovers a
  findings file that was never written. `>|` is for a shell with `noclobber`. Under
  `-s read-only` Codex cannot write files, so the brief tells it to put its complete
  findings in its final message, and the runner copies `last-<lens>.md` to
  `findings-<lens>.md` under a first line `Source: last-message`. A second round writes
  beside the first with a `-2` suffix.
- **Polling.** The runner polls on `pgrep -f '<id>-<n>/last-<lens>.md'` or the process
  id, never the bare file name, which can match a Codex run in another worktree. The
  poll fails closed: if Codex has exited and holds no findings in `last-<lens>.md` or
  the log, the lens runs again. An empty result is never recorded as "no findings".
- **Brief.** It gives this ticket's path, `tickets/CONVENTIONS.md`, the checkpoint's
  questions, and the artefact: the plan at checkpoint 1, `git diff "$base"...HEAD` plus
  the uncommitted changes at checkpoint 2. `base` is the commit step 1 records. It never
  gives the author's reasoning or the hand-back's conclusions.
- **Triage and apply.** Separate fresh subagents check each finding by a command or a
  read and write `ai_tmp/review/<id>-<n>/verdicts.md`. They apply valid fixes one writer
  at a time. Before the lead runs `just check`, it confirms every writer has stopped.
- **Network.** Each checkpoint authorises its model call (§11, "Network").

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `pyproject.toml` | shared, own lines | `openjdk` pinned exactly in `[tool.pixi.dependencies]`, with a comment saying what it is for, the decision it follows and the day both platforms were checked |
| `pixi.lock` | shared, regenerated | relocked by `just lock`, never edited by hand |
| `Justfile` | shared, own lines | the `QUINT_HOME` comment and the `quint` recipe's comment say Apalache runs on pixi's Java; a `JAVA_HOME` export if step 2 shows the launcher reads `JAVA_HOME` at all |
| `docs/decisions/0021-project-managed-quint-cli.md` | shared, own lines | Decision: Java is pinned in pixi; Consequences: the Java download, and port 8822; What would reopen this: the Java clause is met |
| `docs/how-to/develop-locally.md` | shared, own lines | "Quint": `verify` and `compile --target tlaplus` work, run on pixi's Java, and use any server on port 8822 |
| `docs/how-to/maintain-dependencies.md` | shared, own lines | "The Quint CLI": the `openjdk` pin moves with pixi's pins, and a Quint release may need a newer Java |
| `docs/reference/configuration.md` | shared, own lines | the pins table names `openjdk` beside the Quint CLI; a `JAVA_HOME` row only if the `Justfile` exports it |
| `docs/reference/commands.md` | shared, own lines | the `just quint <args>` row: `verify` runs on pixi's Java and uses port 8822 |
| `docs/explanation/security-model.md` | shared, own lines | the Quint CLI exception: Java comes from conda-forge through `pixi.lock`; Apalache's download is still unchecked until AP02 |
| `.gitignore` | shared, own lines | only if step 5 shows `verify` leaves files outside `.quint/` (Apalache may write `_apalache-out/` into the working directory; unconfirmed) |
| `.scratch/apalache/AP01-java-and-verify.md` | ticket | `status:`, the hand-back notes, and a correction to Verification that a control shows is needed, recorded under Departures |
| `.scratch/apalache/README.md` | shared, own lines | AP01's Status cell |

A page the builder finds that also states "Java is not provided" or the like, outside
this list, is handed back under §11, not edited.

## Steps

1. **Worktree and baseline.** Create the worktree on `ticket/ap01-java-and-verify` from
   the base the maintainer names (the README's "Before AP01 starts"). Record the base and
   its full commit id as `base`. Run `just initialize` (network, authorised). Run
   `just check` on the untouched worktree and quote its last line, headed by the commit.
   Record `du -sk .pixi/envs/default` for step 6. For the home-directory probe, record
   whether `~/.quint` exists (`test -e "$HOME/.quint"`, with its exit status). If it
   does, save an inventory that follows a symlinked root and hashes every file:
   `find -L "$HOME/.quint" -type f -exec shasum -a 256 {} +` into
   `ai_tmp/ap01/home-before.raw`, exiting 0, then `LC_ALL=C sort` of it into
   `ai_tmp/ap01/home-before.txt`, as Verification does after the runs. Save `git status --short --ignored` to
   `ai_tmp/ap01/ignored-before.txt`.
2. **Read before writing** (network, authorised). Run `pixi search openjdk` for
   `linux-64` and for `osx-arm64` and quote the newest 21.x build of each (claim 1).
   Download `apalache.tgz` for v0.62.4 from the URL in Context into a new, empty
   `ai_tmp/ap01/download/`, and list it (claim 3). Read `apalache/bin/apalache-mc`:
   record how it finds `java` (`JAVA_HOME`, `PATH`, or both) and any Java version it
   requires, and read the release notes for the minimum Java (claims 2 and 4). Write
   `ai_tmp/ap01/plan.md`: the pin, whether the `Justfile` needs a `JAVA_HOME` export and
   why, the scratch module of step 5, and the edit to each page in Files touched. Nothing
   under `ai_tmp/ap01/download/` is executed.
3. **Adversarial review** (checkpoint 1: conformance and breakage, over
   `ai_tmp/ap01/plan.md`, before any tracked file changes). Questions:
   - Does the plan's account of the launcher match `apalache/bin/apalache-mc`? Would a
     `java` elsewhere on the machine, or a `JAVA_HOME` the contributor's shell sets,
     win over pixi's?
   - Could step 5's run pass while Apalache ran on a Java outside the worktree, or while
     Quint talked to a server it did not start?
   - Is the pin one conda-forge publishes for both platforms?
   - Does any page outside Files touched state that the environment lacks Java?
4. **The pin.** Add `openjdk` to `pyproject.toml`, exact, at the version step 2 chose,
   with its comment. Run `just lock` (network, authorised) and then `just sync`
   (network, authorised). `git diff --stat` must list `pyproject.toml` and `pixi.lock`
   and no other lockfile. If step 2 showed the launcher reads `JAVA_HOME` at all, export
   it in the `Justfile` beside `QUINT_HOME`, pointing into `.pixi/envs/default`, with a
   comment. Otherwise a `JAVA_HOME` in the contributor's shell would pick the Java.
5. **The verify run** (network, authorised: the first run downloads Apalache). Write
   `ai_tmp/ap01/smoke.qnt`:

   ```quint
   module smoke {
     var x: int
     action init = x' = 0
     action step = x' = (x + 1) % 3
     val inRange = x >= 0 and x < 3
     val belowTwo = x < 2
   }
   ```

   Then, in order, quoting each command's output and exit status:
   - the port probe of Verification, with its positive control, showing nothing
     listens on 8822;
   - `just quint verify --invariant=inRange --max-steps=5 ai_tmp/ap01/smoke.qnt`, which
     exits 0 and downloads Apalache into `.quint/apalache-dist-0.62.4/`;
   - while a second such run is in progress, the executable of the Apalache server
     process, showing a `java` under `.pixi/envs/default`. Read the executable's path,
     not its argument list, which shows a bare `java` when the launcher finds it on
     `PATH`: `ps -o comm= -p <pid>` on macOS, `readlink /proc/<pid>/exe` on Linux.
     This is also the positive control for the process probe: `pgrep -fl apalache`
     finds that server while the run is in progress. `QUINT_HOME` is relative, so the
     server's command line may hold a relative path, and a pattern that names `$PWD`
     could never match;
   - the same run with `JAVA_HOME=/nonexistent` set in the environment of `just`. It
     still passes and the executable is still pixi's `java`, so a contributor's
     `JAVA_HOME` cannot choose the Java;
   - `just quint verify --invariant=belowTwo --max-steps=5 ai_tmp/ap01/smoke.qnt`,
     which exits non-zero and reports the violation, so the checker really ran. Read
     from `processApalacheResult` in `dist/src/cliReporting.js`, lines 234 and 246, and
     not yet run: a violation prints `[violation] Found an issue`, a clean run prints
     `[ok] No violation found`. A failure with no trace prints no bracketed marker, only
     Quint's `error:` line.
     Quote the lines the runs print. A non-zero exit without the violation line is a
     failure to start, not a counterexample;
   - the port probe again, and `pgrep -fl apalache`, both finding nothing after Quint
     has exited (claim 5). Any Apalache process on the machine fails this probe, which
     is the intent: one from another worktree would also have answered on port 8822;
   - the home-directory probe of Verification;
   - `git status --short --ignored`, compared with `ai_tmp/ap01/ignored-before.txt`.
     Any new path outside `.quint/` and `ai_tmp/` is something `verify` leaves in the
     worktree. Quote it; if it is not already ignored, add it to `.gitignore` with a
     comment.
6. **Words.** Edit each page in Files touched as the plan says. Record
   `du -sk .pixi/envs/default` again, and put both figures in the hand-back. Run
   `just check-docs`.
7. **Adversarial review** (checkpoint 2: conformance and breakage, over the diff, before
   the final commit). Questions:
   - Does every page in Files touched now agree that Java is pinned, and does any still
     say the environment lacks it?
   - Is `pixi.lock` regenerated rather than edited, and does it carry `openjdk` for both
     platforms?
   - Could Verification pass on a machine where Apalache used a Java outside the
     worktree, or a server on port 8822 that it did not start?
   - Does anything this ticket adds run in `just check` or need the network there?
8. **Commit and hand back.** Commit the work. Rerun Verification and quote it, headed by
   the full commit id. Set `status: done` here and in `.scratch/apalache/README.md`,
   write the hand-back notes, and commit the record. **Stop** before pushing or opening a
   pull request: both need authorisation (`tickets/CONVENTIONS.md` §11, "Separately
   authorised actions").

## Acceptance criteria

- `pyproject.toml` pins `openjdk` exactly, with its comment, and `pixi.lock` holds it
  for `linux-64` and `osx-arm64`. `just lock-check` passes.
- With nothing listening on port 8822 beforehand, `just quint verify` exits 0 on
  `inRange` and non-zero on `belowTwo` with the violation line, and the server's
  executable is a `java` under `.pixi/envs/default`, with and without a bogus
  `JAVA_HOME`.
- After each run, nothing listens on port 8822 and no Apalache process from this
  worktree is left.
- Apalache 0.62.4 sits under `.quint/apalache-dist-0.62.4/`, and `git status --short`
  does not show it.
- The home-directory probe shows `~/.quint` still absent, or, if it existed at step 1,
  holding the same files with the same content. The probe watches `~/.quint` only,
  where Quint writes when `QUINT_HOME` is unset.
- Each of the five claims has its outcome recorded.
- Both checkpoints are recorded as §14's Record asks, with the deltas above.
- `just check` ends with `All checks passed and the worktree is unchanged.`, and
  `git status --short` prints nothing after the commit.

## Verification

Run as one `sh` script; `set -eu` stops it at the first command that fails. Each probe
that passes by printing nothing follows its positive control. `home_existed` is `yes`
or `no`, as step 1 recorded; set it before the script runs.

```sh
set -eu
git rev-parse HEAD

# The port probe. lsof exits 1 when nothing matches, so the status is read directly.
python -c 'import socket,time; s=socket.socket(); s.bind(("127.0.0.1",8822)); s.listen(); time.sleep(5)' &
listener=$!
sleep 1
lsof -nP -iTCP:8822 -sTCP:LISTEN
kill "$listener"
wait "$listener" || true
sleep 1
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi

just quint verify --invariant=inRange --max-steps=5 ai_tmp/ap01/smoke.qnt >| ai_tmp/ap01/ok.log 2>&1
grep -F '[ok] No violation found' ai_tmp/ap01/ok.log
if just quint verify --invariant=belowTwo --max-steps=5 ai_tmp/ap01/smoke.qnt >| ai_tmp/ap01/violation.log 2>&1; then exit 1; fi
grep -F '[violation] Found an issue' ai_tmp/ap01/violation.log

test -x .quint/apalache-dist-0.62.4/apalache/bin/apalache-mc
if lsof -nP -iTCP:8822 -sTCP:LISTEN; then exit 1; else test "$?" -eq 1; fi
if pgrep -fl apalache; then exit 1; else test "$?" -eq 1; fi

# The home probe: absent stays absent; present keeps every file's content.
case "$home_existed" in
  no) test ! -e "$HOME/.quint" ;;
  yes)
    find -L "$HOME/.quint" -type f -exec shasum -a 256 {} + >| ai_tmp/ap01/home-after.raw
    LC_ALL=C sort ai_tmp/ap01/home-after.raw >| ai_tmp/ap01/home-after.txt
    test -s ai_tmp/ap01/home-before.txt
    cmp ai_tmp/ap01/home-before.txt ai_tmp/ap01/home-after.txt ;;
  *) exit 1 ;;
esac

just lock-check
just check-docs
just check
status=$(git status --short)
test -z "$status"
```

`python` is pixi's, first on `PATH` inside the worktree's environment; run the script
with `.pixi/envs/default/bin` first on `PATH`, or through `pixi run sh`. Run it from a
file, `sh ai_tmp/ap01/verify.sh`, never as `sh -c '…'`: the inline text names `apalache`,
so `pgrep -fl apalache` would match the shell running it. The `|| true`
after `wait` covers only the killed control listener, and no claim rests on it. The
two `grep -F` lines read text taken from `dist/src/cliReporting.js`, not from a run: if
step 5's runs print it otherwise (colour codes, say), the builder matches what they
print and says so under Departures.
The server's `java` path is step 5's, not this script's, because it needs a run in
progress. The builder runs each control first and corrects a probe that does not behave
as written, saying so under Departures.

## Hand-back notes

### 1. Base and branch

Base `quint-spec` at `79b8cbdc61094c79feb0e38c0e3ac99a5e056939`. The work ran on
`quint-spec` in this worktree, not on `ticket/ap01-java-and-verify` (Departures, item 1).
The work landed as `5b74af38761f3f4b582b80a96085ab5d7070bf31`.

### 2. Baseline

At `79b8cbd`, `just check` on the untouched worktree, exit 0:

```text
All checks passed and the worktree is unchanged.
```

At `79b8cbd`: `du -sk .pixi/envs/default` gave `345148`. `test -e "$HOME/.quint"` exited
1, so `~/.quint` was absent and `home_existed=no`. `git status --short --ignored` was
saved for step 5.

At `5b74af3`, after the pin: `du -sk .pixi/envs/default` gave `686556`, so the
environment grew from 337 MiB to 670 MiB on macOS. `~/.quint` was still absent.

### 3. Claims

Read at `79b8cbd` (step 2) and run at the commits named.

1. **Holds.** `pixi search -p linux-64 'openjdk 21.*'` gave `openjdk-21.0.13.1-h1602c4f_2`
   as the newest build, and `osx-arm64` gave `openjdk-21.0.13.1-h2da27d0_2` (2026-11-23).
2. **Holds.** Every class file in `apalache.jar` has major version 65 or lower (Java 21),
   and the main class is 65. The launcher supports 21 explicitly. The v0.62.4 release
   notes, read with `gh api repos/apalache-mc/apalache/releases/tags/v0.62.4`, list two
   bug fixes and give no minimum Java. Every run passed on `openjdk version "21.0.13.1"
   2026-11-17 LTS` (sections 5 and 9).
3. **Holds.** `tar -tzf apalache.tgz` exited 0 and listed seven entries under one
   top-level `apalache/`: `apalache/`, `apalache/bin/`, `apalache/bin/apalache-mc`,
   `apalache/bin/apalache-mc.bat`, `apalache/lib/`, `apalache/lib/apalache.jar` and
   `apalache/LICENSE`.
4. **Failed as written.** The launcher runs a bare `java` from `PATH` and never reads
   `JAVA_HOME`, so that half holds. But conda-forge's `openjdk` installs `java` at
   `.pixi/envs/default/lib/jvm/bin/java`, and `.pixi/envs/default/bin/java` does not
   exist. Its activation script would set `JAVA_HOME`, and recipes run no activation. With
   pixi's `bin/` first on `PATH`, `apalache-mc` would therefore run whatever `java` comes
   later. The builder stopped. The maintainer ruled that the `quint` recipe alone prefixes
   `lib/jvm/bin` to `PATH` (Departures, item 3).
5. **Holds.** After every run the spawned pid was gone within the bounded wait.
   `lsof -nP -iTCP:8822 -sTCP:LISTEN` and `pgrep -fl apalache` both exited 1, and each
   followed its positive control (sections 5 and 9).

### 4. What landed

`5b74af3`, `git show --stat`:

```text
 .gitignore                                       |   4 ++++
 Justfile                                         |  13 +++++++++++--
 docs/decisions/0021-project-managed-quint-cli.md |  19 +++++++++++++++----
 docs/explanation/security-model.md               |   5 +++--
 docs/how-to/develop-locally.md                   |   5 ++++-
 docs/how-to/maintain-dependencies.md             |   5 +++++
 docs/reference/commands.md                       |   2 +-
 docs/reference/configuration.md                  |   1 +
 pixi.lock                                        | Bin 38678 -> 64173 bytes
 pyproject.toml                                   |   5 +++++
 10 files changed, 49 insertions(+), 10 deletions(-)
```

- `pyproject.toml`: `openjdk = "==21.0.13.1"`, with a comment naming Apalache, the
  launcher's bare `java`, decision 0021, the class-file ceiling and the day both
  platforms were checked.
- `pixi.lock`: `just lock` regenerated it, and every change is an addition. `linux-64`
  adds `openjdk 21.0.13.1 h1602c4f_2` and 43 other packages (fonts, X11 libraries).
  `osx-arm64` adds `openjdk 21.0.13.1 h2da27d0_2` alone.
- `Justfile`: the `QUINT_HOME` comment and a comment on the `quint` recipe, which now
  reads:

  ```text
  env -u APALACHE_JAR TMPDIR="${TMPDIR:-/tmp}" PATH={{ quote(justfile_directory() / ".pixi" / "envs" / "default" / "lib" / "jvm" / "bin") }}:"$PATH" tools/quint/node_modules/.bin/quint "$@"
  ```

  No `JAVA_HOME` is exported.
- `.gitignore`: `/_apalache-out/`, with a comment (section 5).
- The five pages in Files touched, as the plan said.

### 5. The verify runs

These ran at `79b8cbd` plus the uncommitted pin, lock and recipe. At that point the recipe
had only the `PATH` prefix; checkpoint 2 added `env -u APALACHE_JAR` and the `TMPDIR`
default. Each ran through the corrected driver, which section 8 describes.

**First attempt, failed.** Run 1 downloaded Apalache and passed
(`[ok] No violation found (10008ms).`, pid gone, both probes exit 1). Run 2 stopped:

```text
executable as read: java
FAIL: ok2: ps gave no absolute executable path for pid 84703: 'java'
```

On macOS, `ps -o comm=` shows the bare `java` the launcher `exec`s. The builder
restored a checked `lsof -d txt` read of the executable, removed the distribution that
attempt had downloaded, and reran from a cold `.quint/`.

**Clean run** (`Apalache before run 1: absent`). Both positive controls passed
(`control lsof exit 0`, `control pgrep exit 0`), then both probes printed nothing
(`exit 1`). Per run:

| Run | Result line | Exit | Server |
| --- | --- | --- | --- |
| 1, `inRange`, cold | `Downloading Apalache distribution 0.62.4... done.` then `[ok] No violation found (5876ms).` | 0 | pid 86675 gone |
| 2, `inRange`, live, 20 steps | `[ok] No violation found (3038ms).` | 0 | pid 86764, executable `.pixi/envs/default/lib/jvm/bin/java`, jar `.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar`; gone |
| 3, as 2 with `JAVA_HOME=/nonexistent` | `[ok] No violation found (3057ms).` | 0 | pid 86885, the same executable and jar; gone |
| 4, `belowTwo` | `[violation] Found an issue (3135ms).` | 1 | pid 87246 gone |
| 5, `inRange`, `TMPDIR` unset | `[ok] No violation found (3009ms).` | 0 | pid 87413 gone |

The paths above are worktree-relative; the run printed them absolute. After each run,
and after all of them, `lsof … exit 1` and `pgrep -fl apalache exit 1`. The home probe
printed `home probe: ok` (absent stays absent).

- **`_apalache-out/`.** The server writes a run directory per check under the worktree
  root, for example `_apalache-out/server/<timestamp>/log0.smt`, `detailed.log` and
  `run.txt`. The ignored-files diff showed `?? _apalache-out/`, a new, untracked path
  outside `.quint/` and `ai_tmp/`, so it went into `.gitignore`.
- **`tmp/`.** With `TMPDIR` unset, the launcher created an empty `tmp/` in the worktree
  root (`before run 5: tmp absent`, `after run 5: tmp exists`, `total 0`). Git omits
  empty directories, so it was not a `.gitignore` case. The recipe now defaults `TMPDIR`
  instead (section 7, B5).
- **Round-2 residual risks.** 20 live steps gave enough time to inspect every live run.
  `--out-itf` did not suppress the violation line.

Raw logs stay in `ai_tmp/`.

### 6. Checkpoint 1 record: the plan

**Harness and model.** Codex CLI v0.172.0, `codex exec -m gpt-6.2-sol -s read-only`, in
the corrected form under Adversarial review. Lenses were conformance and breakage, over
`plan.md` and the probe driver. Triage and apply were done by fresh Claude subagents,
one writer at a time. The reviewers rated findings blocker, major or minor, mapped to
high, medium and low. None was a blocker. The lead rated C1/B1 and C2 high, because
each let an acceptance criterion pass while false, so a second round ran.

**Round 1.** 15 findings, 13 distinct (C1 = B1, C4 = B8). 15 valid, 0 invalid.

- C1/B1 (high, by the lead), valid: the JVM probe could check another server. Every run
  now uses `--verbosity=4` and must log Quint's own spawn lines, with no
  `Connecting with existing Apalache server`. The spawned pid must be the only listener,
  and its executable and jar are checked.
- C2 (high, by the lead), valid: the final Verification never proved the runtime or the
  per-run cleanup. `verify.sh` replaces it (Departures, item 4).
- C3, valid: the port control could pass after a failed bind. It now requires the
  control's own pid alone, then that pid gone.
- C4/B8, valid: the README still says pixi has no Java. Handed back (below).
- C5, valid: an empty `~/.quint` would fail `test -s`. Now `test -f` plus `cmp`.
- C6, valid: three plan edits went beyond their evidence. Reworded.
- C7, valid in part: the archive has seven entries, not five; the release notes were owed
  (settled at checkpoint 2, C8).
- B2, valid: a proxy could carry the gRPC call. Runs set `no_grpc_proxy`.
- B3, valid: the driver could exit 0 after failing. It now runs under `set -eu`, with
  `die` and bounded polls.
- B4, valid: `BASH_ENV` could change `PATH`. It is unset, and pixi's `java` is a
  precondition.
- B5, valid: an inherited `APALACHE_JAR` swaps the jar. It is unset, and the `-jar`
  argument is checked.
- B6, valid: the `tmp/` probe looked in the wrong place. It now reads `tmp/` and
  `_apalache-out/` directly.
- B7, valid: the violation run can fetch the Rust evaluator. Its presence is a
  precondition.

**Round 2** (the last). 13 findings, 10 distinct (RB1 = RC1, RB3 = RC3, RB5 = RC4). 10
valid, 0 invalid. It ran with the maintainer's claim-4 ruling applied.

- RC1/RB1 (high), valid: a symlinked distribution passed the jar check. `check_dist` now
  requires regular files at the exact worktree paths.
- RC2, valid: the `lsof` fallback dropped its status. It was removed. Step 5's first
  attempt then showed it was needed, and it was restored with its status checked
  (section 5).
- RC3/RB3, valid: the `-jar` read split a path with spaces. It now matches the argument
  whole.
- RC4/RB5, valid: `wait` had no deadline. Each run now has a watchdog.
- RC5, valid: run 1 could skip the download. The driver refuses a warm `.quint/`.
- RC6, valid: an empty `pgrep` came before its control. The control listener carries
  the word `apalache`.
- RC7, valid: an empty `tmp/` could trigger a `.gitignore` edit. Recorded as evidence
  only.
- RB2, valid: a run could end before it was inspected. Live runs use `--max-steps=20`.
- RB4, valid: a JVM diagnostic could supply the marker. The match is now whole-line,
  the JVM option variables are unset, and `belowTwo` must write `--out-itf`.
- RB6, valid: the plan gave the wrong `mktemp -t` default. Corrected.

**Applied.** All of the above went into `plan.md`, `run-probes.sh` and `verify.sh`,
before any tracked file changed.

**Handed back.** The README introduction (C4/B8). The release notes (C7) went to the
lead and were settled at checkpoint 2. B1's client-side connection attribution went to
Open points.

**Re-run.** The checkpoint's verification is step 5's run (section 5), at `79b8cbd` plus
the uncommitted work.

### 7. Checkpoint 2 record: the diff

**Harness and model.** The same as checkpoint 1, over `git diff 79b8cbd` plus the
uncommitted changes. One round: 14 findings, 11 distinct (C1 = B1, C3 = B2, C2 is B3's
Java half, C7 is B6's README half). None was high. 13 valid, one valid in part (B4),
0 invalid, so no second round ran.

- C1/B1, valid: a missing pixi Java falls through to a system `java`. Handed to AP02's
  approved `just quint` guard.
- C2 / B3 (Java half), valid: the check accepted any `version "21.`. It now requires the
  exact pin from `pyproject.toml`. Applied.
- B3 (Apalache half), valid: the distribution was checked by path only. Every run's log
  must now hold the `# APALACHE version: 0.62.4` banner. Applied.
- C3/B2, valid: the violation run was not tied to its server. It now runs live. Applied;
  the gRPC connection itself is not tied to the pid, which goes to Open points.
- C4, valid: the recipe inherited `APALACHE_JAR`. It now uses `env -u APALACHE_JAR`,
  which the commands page mentions. Applied.
- C5, valid: run 1 inherited the caller's `JAVA_HOME`. It now unsets it. Applied.
- C6, valid: `compile --target tlaplus` shares `connect`. The port caveat now covers
  both commands. Applied.
- C7 / B6 (README half), valid: the README introduction. Handed back to the maintainer.
- B6 (ticket half), valid: this ticket's Context still uses the present tense. Handed
  back.
- C8, valid and settled: the release notes give no minimum Java (claim 2).
- B4, valid in part: an evaluator fetch went unchecked. The driver now rejects the
  `Fetching Rust evaluator` line; a network-denied run goes to Open points.
- B5, valid: an unset `TMPDIR` creates `tmp/`. The recipe passes `TMPDIR="${TMPDIR:-/tmp}"`.
  Applied.

**Applied** to the `Justfile` recipe, `docs/how-to/develop-locally.md`,
`docs/reference/commands.md` and `verify.sh`, before `5b74af3`.

**Re-run.** Section 9, at `5b74af3`.

**Handed back to the maintainer (§11).** `.scratch/apalache/README.md:3-7` and this
ticket's Context (lines 41-43) still describe the pre-AP01 machine in the present tense,
saying the pixi environment has no Java. Both are outside the record lines this ticket
may edit.

**Handed to AP02.** A refusal in `just quint` for `verify` and `compile --target tlaplus`
when pixi's Java is missing. Without it, a worktree that pulled this commit without
running `just sync` falls through to a system `java`. AP02's approved guard covers the
same two commands, so the refusal belongs there.

### 8. Departures

1. **Branch and worktree.** The work ran on `quint-spec`, in this worktree, with no
   ticket branch, no new worktree and no rerun of `just initialize`. The maintainer
   ruled this on 2026-11-24.
2. **Reviewer model.** `gpt-6.2-sol`, not `gpt-6-astra`, by the user's choice. The
   harness is still Codex, as this ticket prefers.
3. **Claim 4 and the `PATH` prefix.** Claim 4 failed (section 3). Under the maintainer's
   ruling, the `quint` recipe alone puts `.pixi/envs/default/lib/jvm/bin` first on
   `PATH`, so no other recipe sees a Java. The `Justfile` row's "a `JAVA_HOME` export"
   does not apply: the launcher never reads `JAVA_HOME`, and no `JAVA_HOME` is exported.
4. **Verification corrected.** The checkpoints required it (C1/B1, C2 and their round-2
   and checkpoint-2 sequels). The ticket's script is replaced by `verify.sh`, which adds
   the following:
   - each run at `--verbosity=4`, with its log required to hold Quint's own spawn lines
     and not `Connecting with existing Apalache server`;
   - the spawned pid as the only listener on 8822, its executable resolved to pixi's
     `lib/jvm/bin/java`, and its jar to this worktree's
     `.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar`;
   - `java -version` checked against the exact `openjdk` pin, and the
     `APALACHE version: 0.62.4` banner required;
   - no Apalache download line and no evaluator fetch line in the logs;
   - port and process cleanup after each run, not only at the end;
   - `APALACHE_JAR`, `BASH_ENV` and the JVM option variables unset, and `no_grpc_proxy`
     set;
   - a watchdog on every run;
   - live runs at `--max-steps=20`, and `belowTwo` writing `--out-itf`.
5. **Probe order.** The positive controls (the control listener's own pid, alone on 8822
   and listed by `pgrep`) run before the first empty probe. The executable, which step 5
   set beside the second run, is read during every live run. Step 5 added run 5 at the
   end.
6. **The recipe's `env -u APALACHE_JAR` and `TMPDIR` default.** Both were added at
   checkpoint 2 (C4, B5). They sit within the `Justfile` row as the `quint` recipe's own
   lines.
7. **The unset-`TMPDIR` probe.** Step 5's run 5 is extra. It found the launcher's `tmp/`
   fallback (section 5).

### 9. Verification

`verify.sh` ran at `5b74af3` with `home_existed=no` and exited 0. Excerpts:

```text
5b74af38761f3f4b582b80a96085ab5d7070bf31
openjdk version "21.0.13.1" 2026-11-17 LTS
control lsof exit 0
control pgrep exit 0
lsof -iTCP:8822 -sTCP:LISTEN exit 1
pgrep -fl apalache exit 1
ok: server pid 4876; executable …/.pixi/envs/default/lib/jvm/bin/java -> …/.pixi/envs/default/lib/jvm/bin/java; jar …/.quint/apalache-dist-0.62.4/apalache/bin/../lib/apalache.jar -> …/.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar
# APALACHE version: 0.62.4 | build: 7954daf
[ok] No violation found (3342ms).
ok: server pid 4876 gone
ok-javahome: server pid 5085; executable …/.pixi/envs/default/lib/jvm/bin/java -> …/.pixi/envs/default/lib/jvm/bin/java; jar … -> …/.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar
[ok] No violation found (3026ms).
ok-javahome: server pid 5085 gone
violation: server pid 5206; executable …/.pixi/envs/default/lib/jvm/bin/java -> …/.pixi/envs/default/lib/jvm/bin/java; jar … -> …/.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar
violation exit 1
[violation] Found an issue (3110ms).
violation: server pid 5206 gone
lsof -iTCP:8822 -sTCP:LISTEN exit 1
pgrep -fl apalache exit 1
home probe (no): ok
✔ Lock-file was already up-to-date
Validated 49 pages and 50 canonical topics.
All checks passed and the worktree is unchanged.
Verification passed.
```

`…` stands for the worktree path. Each run printed `# APALACHE version: 0.62.4` and
`openjdk` reported `version "21.0.13.1"`, the pin. The Linux branch of the executable and
`-jar` reads (`/proc/<pid>/exe`, `/proc/<pid>/cmdline`) was checked by `sh -n` only:
every run here was on macOS. The raw logs stay in `ai_tmp/`.

## Open points

- **Java in every CI job.** `just sync` runs `pixi install --frozen` in every CI job, so
  the default environment's Java is downloaded and cached in jobs that never run Quint.
  Step 6's two sizes show the cost: the default environment grew from 337 MiB to
  670 MiB on macOS, and `linux-64` locks 43 more packages beside `openjdk`. A pixi feature with an environment of its own would
  keep Java out of the default environment, at the price of a second `PATH` entry in the
  `Justfile`. The maintainer decides if the cost matters.
- **Port 8822.** A server another worktree started answers this worktree's
  `just quint verify`, with that worktree's Apalache and Java. Passing a free
  `--server-endpoint` per run, or refusing to run while the port is taken, would close it.
  Neither is in scope.
- **Quint's connection and the spawned pid.** Verification proves that the pid Quint
  spawned is the only listener on 8822, but not that Quint's gRPC connection reached that
  pid (checkpoint 1 B1, checkpoint 2 C3/B2). An `lsof -i` read of the node process's
  client socket would tie them. It sits beside "Port 8822".
- **A network-denied Verification.** Verification rejects Quint's download and fetch
  lines but does not deny the network (checkpoint 2 B4). macOS has no simple per-process
  network denial without new tooling. A run under §11's denial, with its negative
  control, would show that a warm `.quint/` needs no network.
