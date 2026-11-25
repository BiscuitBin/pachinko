---
id: AP01
title: "A pinned Java runtime, and a working just quint verify"
status: todo
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

### 2. Baseline

### 3. Claims

### 4. What landed

### 5. The verify runs

### 6. Checkpoint 1 record: the plan

### 7. Checkpoint 2 record: the diff

### 8. Departures

### 9. Verification

## Open points

- **Java in every CI job.** `just sync` runs `pixi install --frozen` in every CI job, so
  the default environment's Java is downloaded and cached in jobs that never run Quint.
  Step 6's two sizes show the cost. A pixi feature with an environment of its own would
  keep Java out of the default environment, at the price of a second `PATH` entry in the
  `Justfile`. The maintainer decides if the cost matters.
- **Port 8822.** A server another worktree started answers this worktree's
  `just quint verify`, with that worktree's Apalache and Java. Passing a free
  `--server-endpoint` per run, or refusing to run while the port is taken, would close it.
  Neither is in scope.
