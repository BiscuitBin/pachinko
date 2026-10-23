---
id: T10
title: "Metrics and the effects boundary: rustqual over both crates, the libpachinko bans, the probe"
status: done
depends_on: [T00, T02]
parallel_with: [T01, T05, T06, T07]
branch: ticket/t10-metrics-and-effects
estimated_size: L
---

# T10: Metrics and the effects boundary: rustqual over both crates, the libpachinko bans, the probe

## Context

Decision 3 (`CONVENTIONS.md` §1) is that libpachinko never touches the clock, randomness,
the filesystem, the network, the environment or threads, and that this is enforced
mechanically. L got that guarantee for free from `#![no_std]`; this repository is std, so
the effects have to be named and banned, in libpachinko only, by a gate that has been
shown to fire. That is gate 8, `just effects`, which runs `scripts/check-effects.sh`.
T00's script checks only libpachinko's dependency tree; this lane makes it complete.

Gate 7, `just metrics`, already runs rustqual over both crates with L's thresholds
(`CONVENTIONS.md` §4). This lane makes `rustqual.toml` final for this repository.

`CONVENTIONS.md` §9 is the design: the bans, the candidate mechanisms, and the three
things the script must do. §1 facts 3 and 4 are why the obvious mechanisms are not
enough on their own. §12 claims 2, 11 and 12 are this lane's to settle. L
`tickets/T22-metrics-gate.md` is the procedure for rustqual and for the probe shape (a
fixture that breaks each rule once, and a recipe that fails unless every rule fires);
L `Justfile`'s `metrics` recipe is the probe's shell.

T10 waits for T02 because the clippy configuration is T02's: a libpachinko
`clippy.toml`, if this lane chooses one, must restate T02's final root keys.

## Goal

`just effects` fails on any banned crate in libpachinko's normal dependency tree and on
any banned std API in libpachinko's source; its probe over
`tests/fixtures/effects-violation/` sees every ban fire exactly once; it reports nothing
on `crates/pachinko`; and `just metrics` is green with a `rustqual.toml` whose every
setting is chosen. Both gates are green in `just check`.

## Non-goals

- No effect trait in libpachinko. The traits arrive with the specifications.
- No `Justfile` change: `effects` runs the script and the script is this lane's.
- No rustqual module-boundary rules: there are no modules to bound (decision 0012).
- No change to `deny.toml` or the root `clippy.toml`; a change needed there is handed
  to T02 or, after T02 has merged, made in a `main` follow-up.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `scripts/check-effects.sh` | final | the three checks of `CONVENTIONS.md` §9 |
| `tests/fixtures/effects-violation/` | new | a crate, its path-dependency stubs and its `Cargo.lock`, breaking each ban once (step 4) |
| `crates/libpachinko/clippy.toml` | new | only if step 3 chooses clippy |
| `rustqual-effects.toml` | new | only if step 3 chooses rustqual |
| `rustqual.toml` | final | L's thresholds, the comments rewritten for two crates |
| `tickets/T10-metrics-and-effects.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 and T02 have merged.
2. **Settle `CONVENTIONS.md` §12 claim 2,** in a scratch copy of the repository under
   `ai_tmp/`, for each mechanism in turn:
   - clippy: put `disallowed-methods = [{ path = "std::time::SystemTime::now" }]` in
     `crates/libpachinko/clippy.toml` and a call in libpachinko; does `just clippy` fail
     on libpachinko, pass on the same call in pachinko, and still apply the root file's
     keys (`too-many-lines-threshold`) to libpachinko? Clippy is documented to use the
     first `clippy.toml` it finds walking up from the crate's manifest directory; confirm
     whether it merges or shadows.
   - rustqual: an `[[architecture.pattern]]` with `forbid_path_prefix = ["std::fs"]`, run
     with `rustqual crates/libpachinko --config rustqual-effects.toml`; does it catch `use std::fs;
     fs::read(..)` as well as `std::fs::read(..)`, and a glob import?
   - both: does each catch `i.elapsed()` on an `Instant` passed in as a parameter, a
     method call with no path for a prefix rule to match (`CONVENTIONS.md` §9)?
3. **Choose.** The crate bans are the `cargo tree` check (it is already T00's). For the
   std APIs, choose the mechanism that catches every form step 2 tried, with the fewest
   files. Record the evidence and the choice in the hand-back notes; T11 carries it into
   `architecture.md` and decision 0008.

   **Adversarial review** (`CONVENTIONS.md` §14): step 2's evidence for `CONVENTIONS.md`
   §12 claim 2 and the choice made on it, before step 4 builds the fixture on that
   choice. Ask: which form of a banned API step 2 did not try; whether a libpachinko
   `clippy.toml` shadows the root file's keys, and what libpachinko loses if it does.
4. **The fixture.** `tests/fixtures/effects-violation/` is a crate the production
   `cargo tree` line can walk offline, so the probe exercises the same traversal and
   matching that guard libpachinko:
   - Its `Cargo.toml` carries an empty `[workspace]` table, so cargo treats it as its own
     workspace root and not a stray member of this one (`members = ["crates/*"]` already
     leaves it out of the build).
   - Each banned crate is a path-dependency stub under `deps/<name>/`: a `Cargo.toml`
     naming the package and an empty `src/lib.rs` (cargo refuses a manifest with no
     target). Path dependencies need no registry, so nothing is fetched.
   - Every banned name but one is a direct dependency of the fixture. The last arrives
     transitively, through a non-banned stub, `deps/wrapper/`, that depends on it, so the
     probe shows the `cargo tree` line sees past a direct parent.
   - Its `Cargo.lock` is generated once with
     `cargo generate-lockfile --offline --manifest-path tests/fixtures/effects-violation/Cargo.toml`
     and committed, so `--locked` holds and `check-clean` sees no new file.
   - Its `src/` breaks each std ban exactly once, in the form the chosen tool resolves,
     plus one aliased form (`use std::fs as f;`) where the tool could miss it.

   The script runs the identical `cargo tree -e normal --prefix none --locked --offline`
   line over the fixture with `--manifest-path`, through the same matching code it uses
   for libpachinko. There is no manifest-text fallback: if `cargo tree` cannot walk the
   fixture offline, stop and hand back. `just toml-check` and the hooks see the new
   `Cargo.toml` files, so format them with taplo.
5. **The script.** Final `scripts/check-effects.sh`: POSIX `sh`, `set -eu`; the ban list
   in one place at the top, one name per line with a reason; the three checks of
   `CONVENTIONS.md` §9 in order; on failure, print the finding and what was expected, in
   the shape of L's `metrics` probe output. Offline (`CONVENTIONS.md` §12 claim 12).
6. **`rustqual.toml`.** L's thresholds and every comment that still holds; comments that
   name L's modules rewritten. Confirm `just metrics` is green on both crates
   (`CONVENTIONS.md` §12 claim 11).
7. **Red, then green.** In scratch commits, add each banned API to libpachinko in turn and
   confirm `just effects` fails naming it; remove one break from the fixture and confirm
   the probe fails naming the silent ban; add a banned API to pachinko and confirm
   nothing fires. Quote each.
8. `just check`, the final adversarial review, `just check` again if fixes landed, then
   commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff and step 7's red runs.
   Ask: which ban could go silent without the probe noticing; whether `just effects` runs
   offline on a machine with a cold cache; whether anything in it fires on, or changes,
   the pachinko crate.

## Acceptance criteria

- `just effects` and `just metrics` are green.
- The hand-back notes quote, for every ban, `just effects` failing when libpachinko
  breaks it, and the probe failing when the fixture stops breaking it.
- Every banned crate name, the transitive one included, appears exactly once in the
  `cargo tree` output the probe reads for the fixture; the hand-back notes quote it.
- A banned API in `crates/pachinko` does not fail `just effects`.
- `just effects` passes under `CONVENTIONS.md` §11's network denial, with both
  negative-control probes failing.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
just effects
just metrics
# macOS; on Linux, unshare -rn in place of sandbox-exec (CONVENTIONS.md §11)
deny='(version 1)(allow default)(deny network*)'
sandbox-exec -p "$deny" curl -fsS --max-time 10 https://static.rust-lang.org; echo "exit $?"
sandbox-exec -p "$deny" curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1; echo "exit $?"
sandbox-exec -p "$deny" just effects; echo "exit $?"
just check
```

## Hand-back notes

Executed in the Supacode worktree `T10-metrics-and-effects`, on its branch of the same
name, not on `ticket/t10-metrics-and-effects`; rename the branch before the pull
request if the name matters. Nothing was pushed, and `git fetch` and the rebase onto
`origin/main` that §11 asks for before a push have not run: pushing was not
authorised. `just initialize` was not needed; `.pixi/` and `.tools/` were present.
The commits are `51303ae` (the gate) and `1169dca` (the final review's fixes), then
the commit that adds these notes.

### Baseline: `b4e452a`

```text
$ just check
...
All checks passed and the worktree is unchanged.
```

### Step 2: §12 claim 2, in a scratch copy at `b4e452a`

The script is `ai_tmp/t10/step2.sh` (not committed). It copies the tree into
`ai_tmp/t10/repo`, and runs clippy 0.1.99 (toolchain 1.99.1) and rustqual 1.8.3.

**Clippy reads `crates/libpachinko/clippy.toml` for libpachinko alone, and shadows the
root file rather than merging with it.** The crate file held only
`disallowed-methods = [{ path = "std::time::SystemTime::now", reason = "effect" }]`.
libpachinko gained a 72-line function, a `HashMap` and a `SystemTime::now()` call;
pachinko gained the same `SystemTime::now()` call.

```text
### A/B: cargo clippy --workspace --all-targets -D warnings, with crates/libpachinko/clippy.toml
   --> crates/libpachinko/src/lib.rs:108:5
error: use of a disallowed method `std::time::SystemTime::now`
exit 101
### A/B: cargo clippy --workspace --all-targets -D warnings, without crates/libpachinko/clippy.toml
   --> crates/libpachinko/src/lib.rs:102:5
  --> crates/libpachinko/src/lib.rs:24:1
error: this function has too many lines (72/60)
error: use of a disallowed type `std::collections::HashMap`
exit 101
### B: same crate file, libpachinko clean, the SystemTime::now call left in pachinko
1
exit 0
```

With the crate file present, only the effect ban fires. The root file's
`too-many-lines-threshold` (60) and its `HashMap` ban both go silent, and the same
call in pachinko fires nothing. Shadowing therefore costs libpachinko every root key:
the four thresholds, the `HashMap`/`HashSet` ban and the six `allow-*-in-tests`
keys. The workspace lint table in `Cargo.toml` still applies; only clippy's
configuration is replaced. So the crate file restates the root file line for line,
and check 0 of the script fails when the two differ by more than the effect
entries.

**Clippy takes item paths, not modules (§1 fact 4).** A module path is a warning,
not an error, even under `-D warnings`, so a mistyped entry is silently inert:

```text
### D: clippy given a module path
error: could not compile `libpachinko` (lib) due to 1 previous error; 1 warning emitted
error: docs for function returning `Result` missing `# Errors` section
warning: `libpachinko` (lib) generated 1 warning
warning: expected a function, found a module
exit 101
```

(The `# Errors` error is the probe function's own doc lint.) That is why the fixture
must trip every entry.

**rustqual, `forbid_path_prefix = ["std::fs"]`, plus `forbid_function_call` and
`forbid_method_call` rules, over eleven forms** (`ai_tmp/t10/rq/src/f01.rs` to
`f11.rs`: `use std::fs; fs::read`, `std::fs::read`, `use std::fs::*`,
`use std::fs as f`, `i.elapsed()` on a parameter, `::std::fs::read`,
`use std as s; s::fs::read`, `use std::*; fs::read`, a `macro_rules!` body,
`use std::time::Instant; Instant::now()`, `use std::time::{Instant as I}; I::now()`):

```text
file=src/f01.rs,line=1::architecture/pattern/std_fs — path "std::fs": fs
file=src/f02.rs,line=1::architecture/pattern/std_fs — path "std::fs::read": fs
file=src/f03.rs,line=1::architecture/pattern/std_fs — path "std::fs::*": fs
file=src/f04.rs,line=1::architecture/pattern/std_fs — path "std::fs": fs
file=src/f05.rs,line=1::architecture/pattern/elapsed_by_name — direct method call elapsed: elapsed
file=src/f06.rs,line=1::architecture/pattern/std_fs — path "std::fs::read": fs
file=src/f10.rs,line=1::architecture/pattern/instant_now_call — call Instant::now: now
rustqual exit 1
rustqual exit 1
```

It misses f07, f08, f09 and f11. It catches f10 only through a separate
`forbid_function_call` rule on the bare `Instant::now`, and f05 only through
`forbid_method_call = ["elapsed"]`, which bans the name `elapsed` on every type,
domain types included. The same forms under clippy:

```text
warning: use of a disallowed method `std::fs::read`   --> src/f01.rs:1:35
warning: use of a disallowed method `std::fs::read`   --> src/f02.rs:1:22
warning: use of a disallowed method `std::fs::read`   --> src/f03.rs:1:38
warning: use of a disallowed method `std::fs::read`   --> src/f04.rs:1:40
warning: use of a disallowed method `std::fs::read`   --> src/f06.rs:1:22
warning: use of a disallowed method `std::fs::read`   --> src/f07.rs:1:36
warning: use of a disallowed method `std::fs::read`   --> src/f08.rs:1:34
warning: use of a disallowed method `std::fs::read`   --> src/f09.rs:1:26
warning: use of a disallowed method `std::time::Instant::elapsed`   --> src/f05.rs:1:45
warning: use of a disallowed method `std::time::Instant::now`   --> src/f10.rs:1:49
warning: use of a disallowed method `std::time::Instant::now`   --> src/f11.rs:1:50
clippy exit 0
clippy exit 0
```

All eleven fire, each naming the configured path. The checkpoint-1 triage
added function-item forms (`let c: fn() -> Instant = Instant::now`, `.map(std::fs::read)`,
a `static` initialiser): clippy fires on each.

### Step 3: the choice

**Clippy alone, in `crates/libpachinko/clippy.toml`.** It is the one mechanism
that catches every form step 2 tried, because it resolves names instead of matching
text. It needs one file, as rustqual would. rustqual is not used for the bans, and
there is no `rustqual-effects.toml`.

The file restates the root `clippy.toml` verbatim and adds 108 entries, each with a
reason beginning `effect:`:

- every stable function of `std::fs`, `std::env` and `std::process` in
  `disallowed-methods`;
- every stable struct, enum and trait of `std::fs`, `std::env`, `std::net` and
  `std::process` in `disallowed-types`; a type entry also bans the type's
  associated functions and constants (`File::open`, `ExitCode::SUCCESS`);
- `SystemTime::{now,elapsed}`, `Instant::{now,elapsed}`, `std::thread::{spawn,scope,sleep}`
  and `std::thread::Builder` from §9, with `std::thread::sleep_ms`, the deprecated
  form of `sleep`, beside them;
- additions for effects reached outside those modules, from checkpoint 1's finding
  B1: `std::path::Path`'s ten filesystem queries (`exists`, `metadata`,
  `read_dir` and the rest; `PathBuf` reaches them through `Deref`),
  `std::path::absolute` (it reads the current directory),
  `std::os::unix::fs::{symlink,chown,fchown,lchown,chroot}`,
  `std::os::unix::net::{UnixStream,UnixListener,UnixDatagram}` and
  `std::net::ToSocketAddrs::to_socket_addrs` (DNS).

The inventory came from the `#[stable]` items in the 1.98.1 `library/std/src`
(`fs.rs`, `env.rs`, `process.rs`, `net/mod.rs`, `thread/mod.rs`,
`os/unix/{fs,net}`), and the fixture compiles every entry on 1.99.1. Unstable
items (`Ipv6MulticastScope`, `IntoIncoming`, `hostname`) are left out, because
stable code cannot name them. Only unix paths are listed: a path for another
platform does not resolve here and only warns.

`set_var` and `remove_var` are banned too, although `forbid(unsafe_code)` already
stops a call. The fixture takes them as values, so it needs no `unsafe`.

Deviation from step 5's "the ban list in one place at the top": the crate bans are
at the top of the script. The std-API bans live in `crates/libpachinko/clippy.toml`,
because that is where clippy reads them, and the script extracts the `effect:`
entries from it. Each list is held in one place.

### Checkpoint 1 review (§14): step 3's evidence and choice

Reviewers: Codex CLI 0.163.0, `codex exec -m gpt-6-astra -s read-only`, one run per
lens. The brief held the ticket, CONVENTIONS, `ai_tmp/review/T10-1/choice.md` (the
choice and the planned shape) and the step-2 script and output. Triage: seven fresh
Claude Opus 5.5 subagents, one per finding, each checking by experiment under
`ai_tmp/review/T10-1/triage-<id>/`. Raw files are under `ai_tmp/review/T10-1/`.

| Id | Finding | Verdict | Applied |
| --- | --- | --- | --- |
| C1 (medium) | The inventory omits `std::env::consts`, so §9's "everything under `std::env`" is not met | valid: clippy cannot ban a constant at all ("expected a function, found a constant", a warning) | handed back (Open points) |
| C2 (medium) | The planned clippy run lacks `--all-features` | valid: a feature-gated call is not compiled | `--all-features` on every run, and on `cargo tree` |
| C3 (low) | The evidence used direct cargo, not `just clippy` and `rustqual crates/libpachinko` | invalid: re-run with the prescribed commands, same results | none |
| B1 (high) | Filesystem and socket effects reachable outside the four modules (`Path::exists`, `std::os::unix::fs::symlink`, `UnixStream`) | valid: 0 diagnostics for seven such calls | the additions listed above |
| B2 (medium) | A banned function taken as a value is unproven | invalid: clippy fires on every value form tried | none |
| B3 (medium) | No `--all-features` | valid, a duplicate of C2 | as C2 |
| B4 (medium) | Nothing guards the copied root keys against drift | valid | check 0 in the script |

No high finding stayed open after the fixes (B1's fix is entry additions, each
proven by the fixture), so no second round ran.

### The fixture's tree, read by the probe: `1169dca`

Every banned crate appears exactly once. `socket2` appears only through `wrapper`,
and `getrandom` only under `[target.'cfg(windows)'.dependencies]`, which shows the
tree covers targets other than the host:

```text
$ cargo tree --manifest-path tests/fixtures/effects-violation/Cargo.toml -e normal --prefix none --all-features --target all --locked --offline
effects-violation v0.1.0 (tests/fixtures/effects-violation)
async-std v0.1.0 (tests/fixtures/effects-violation/deps/async-std)
getrandom v0.1.0 (tests/fixtures/effects-violation/deps/getrandom)
mio v0.1.0 (tests/fixtures/effects-violation/deps/mio)
rand v0.1.0 (tests/fixtures/effects-violation/deps/rand)
rand_core v0.1.0 (tests/fixtures/effects-violation/deps/rand_core)
smol v0.1.0 (tests/fixtures/effects-violation/deps/smol)
tokio v0.1.0 (tests/fixtures/effects-violation/deps/tokio)
wrapper v0.1.0 (tests/fixtures/effects-violation/deps/wrapper)
socket2 v0.1.0 (tests/fixtures/effects-violation/deps/socket2)
Exit status: 0
```

### Step 7: red runs, `1169dca`

The harness is `ai_tmp/t10/red.py` (not committed); its log and results are
`ai_tmp/t10/red.log` and `ai_tmp/t10/red/results.json`. Each case resets a scratch
copy of `git archive HEAD`, applies one change and runs `just effects` there. Of
341 cases, 341 behaved as expected. An `OK` requires the exit status and the exact
named set: the fixture tags for API cases, the one crate for crate cases.

An earlier run of the harness reset files with `rsync -a`, which restores old
mtimes. Cargo then took the reverted libpachinko as fresh and replayed the mutated
build's warnings. That is a harness artefact, since `git checkout` and editors stamp
the current time, but it is recorded under Open points.

Representative output, each in full from the first `effects:` line:

```text
# socket2 as a normal dependency of libpachinko
effects: libpachinko depends on a banned crate:
socket2
want none
error: recipe `effects` failed on line 187 with exit code 1

# use std::fs as f; f::read("a") in libpachinko
effects: clippy --all-features over libpachinko exited 0, want 0
banned APIs used:
std::fs::read
want none
error: recipe `effects` failed on line 187 with exit code 1

# fixture: fn instant_elapsed() {}, its tag kept
effects: the API probe exited 0, want 0, and must fire every ban exactly once
silent:
std::time::Instant::elapsed
fired more than once:

error: recipe `effects` failed on line 187 with exit code 1

# fixture: the fs_write line deleted with its tag
effects: the fixture's // ban: tags and the effect entries in crates/libpachinko/clippy.toml disagree
tagged, not in crates/libpachinko/clippy.toml:

in crates/libpachinko/clippy.toml, not tagged:
std::fs::write
error: recipe `effects` failed on line 187 with exit code 1

# fixture: wrapper no longer depends on socket2
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
crates seen:
async-std
getrandom
mio
rand
rand_core
smol
tokio
error: recipe `effects` failed on line 187 with exit code 1

# pachinko: SystemTime::now, env::var_os, thread::spawn, fs::read and a tokio stub dependency
sh scripts/check-effects.sh
effects: libpachinko names none of 8 banned crates and uses none of 108 banned std APIs;
effects: the fixture trips every ban exactly once, and pachinko trips none.
exit 0

# a banned call under #[cfg(not(feature = "injected-clock"))] in libpachinko
effects: clippy --no-default-features over libpachinko exited 0, want 0
banned APIs used:
std::time::SystemTime::now
want none
error: recipe `effects` failed on line 187 with exit code 1

# tokio under [target.'cfg(windows)'.dependencies] of libpachinko
effects: libpachinko depends on a banned crate:
tokio
want none
error: recipe `effects` failed on line 187 with exit code 1

# the std::fs::read entry deleted from crates/libpachinko/clippy.toml
effects: the fixture's // ban: tags and the effect entries in crates/libpachinko/clippy.toml disagree
tagged, not in crates/libpachinko/clippy.toml:
std::fs::read
in crates/libpachinko/clippy.toml, not tagged:

error: recipe `effects` failed on line 187 with exit code 1

# rand deleted from the script's crate list
effects: the crate probe did not see each banned crate exactly once
crates banned:
async-std
getrandom
mio
rand_core
smol
socket2
tokio
crates seen:
async-std
getrandom
mio
rand
rand_core
smol
socket2
tokio
error: recipe `effects` failed on line 187 with exit code 1

# the root clippy.toml's too-many-lines-threshold 60 -> 50, libpachinko's left at 60
effects: crates/libpachinko/clippy.toml differs from clippy.toml beyond its effect entries.
14c14
< too-many-lines-threshold = 50
---
> too-many-lines-threshold = 60
clippy reads only the crate's file for libpachinko; restate the root file there.
error: recipe `effects` failed on line 187 with exit code 1
```

A `SystemTime::now` behind a positive `#[cfg(feature = "clock")]` also fails, through
the `--all-features` pass.

Every ban, both ways. Breaking a ban in libpachinko fails `just effects` naming it;
silencing the fixture's break fails the probe naming it as silent; deleting the
break with its tag fails the inventory check naming it. `set_var` and `remove_var`
moved into libpachinko also fail compilation under `forbid(unsafe_code)` (clippy exit
101); the ban is named all the same.

| Crate | Added to libpachinko | `just effects` | Removed from the fixture | `just effects` |
| --- | --- | --- | --- | --- |
| `rand` | names `rand` | exit 1 | seen without `rand` (7 of 8) | exit 1 |
| `rand_core` | names `rand_core` | exit 1 | seen without `rand_core` (7 of 8) | exit 1 |
| `getrandom` | names `getrandom` | exit 1 | seen without `getrandom` (7 of 8) | exit 1 |
| `tokio` | names `tokio` | exit 1 | seen without `tokio` (7 of 8) | exit 1 |
| `async-std` | names `async-std` | exit 1 | seen without `async-std` (7 of 8) | exit 1 |
| `smol` | names `smol` | exit 1 | seen without `smol` (7 of 8) | exit 1 |
| `mio` | names `mio` | exit 1 | seen without `mio` (7 of 8) | exit 1 |
| `socket2` | names `socket2` | exit 1 | seen without `socket2` (7 of 8) | exit 1 |

| Ban | Break moved into libpachinko | `just effects` | Break silenced in the fixture (tag kept) | `just effects` | Break deleted with its tag | `just effects` |
| --- | --- | --- | --- | --- | --- | --- |
| `std::time::SystemTime::now` | `system_time_now` names `std::time::SystemTime::now` | exit 1 | silent: `std::time::SystemTime::now` | exit 1 | not tagged: `std::time::SystemTime::now` | exit 1 |
| `std::time::SystemTime::elapsed` | `system_time_elapsed` names `std::time::SystemTime::elapsed` | exit 1 | silent: `std::time::SystemTime::elapsed` | exit 1 | not tagged: `std::time::SystemTime::elapsed` | exit 1 |
| `std::time::Instant::now` | `instant_now` names `std::time::Instant::now` | exit 1 | silent: `std::time::Instant::now` | exit 1 | not tagged: `std::time::Instant::now` | exit 1 |
| `std::time::Instant::elapsed` | `instant_elapsed` names `std::time::Instant::elapsed` | exit 1 | silent: `std::time::Instant::elapsed` | exit 1 | not tagged: `std::time::Instant::elapsed` | exit 1 |
| `std::thread::spawn` | `thread_spawn` names `std::thread::spawn` | exit 1 | silent: `std::thread::spawn` | exit 1 | not tagged: `std::thread::spawn` | exit 1 |
| `std::thread::scope` | `thread_scope` names `std::thread::scope` | exit 1 | silent: `std::thread::scope` | exit 1 | not tagged: `std::thread::scope` | exit 1 |
| `std::thread::sleep` | `thread_sleep` names `std::thread::sleep` | exit 1 | silent: `std::thread::sleep` | exit 1 | not tagged: `std::thread::sleep` | exit 1 |
| `std::thread::sleep_ms` | `thread_sleep_ms` names `std::thread::sleep_ms` | exit 1 | silent: `std::thread::sleep_ms` | exit 1 | not tagged: `std::thread::sleep_ms` | exit 1 |
| `std::thread::Builder` | `thread_builder` names `std::thread::Builder` | exit 1 | silent: `std::thread::Builder` | exit 1 | not tagged: `std::thread::Builder` | exit 1 |
| `std::fs::canonicalize` | `fs_canonicalize` names `std::fs::canonicalize` | exit 1 | silent: `std::fs::canonicalize` | exit 1 | not tagged: `std::fs::canonicalize` | exit 1 |
| `std::fs::copy` | `fs_copy` names `std::fs::copy` | exit 1 | silent: `std::fs::copy` | exit 1 | not tagged: `std::fs::copy` | exit 1 |
| `std::fs::create_dir` | `fs_create_dir` names `std::fs::create_dir` | exit 1 | silent: `std::fs::create_dir` | exit 1 | not tagged: `std::fs::create_dir` | exit 1 |
| `std::fs::create_dir_all` | `fs_create_dir_all` names `std::fs::create_dir_all` | exit 1 | silent: `std::fs::create_dir_all` | exit 1 | not tagged: `std::fs::create_dir_all` | exit 1 |
| `std::fs::exists` | `fs_exists` names `std::fs::exists` | exit 1 | silent: `std::fs::exists` | exit 1 | not tagged: `std::fs::exists` | exit 1 |
| `std::fs::hard_link` | `fs_hard_link` names `std::fs::hard_link` | exit 1 | silent: `std::fs::hard_link` | exit 1 | not tagged: `std::fs::hard_link` | exit 1 |
| `std::fs::metadata` | `fs_metadata` names `std::fs::metadata` | exit 1 | silent: `std::fs::metadata` | exit 1 | not tagged: `std::fs::metadata` | exit 1 |
| `std::fs::read` | `fs_read` names `std::fs::read` | exit 1 | silent: `std::fs::read` | exit 1 | not tagged: `std::fs::read` | exit 1 |
| `std::fs::read_dir` | `fs_read_dir` names `std::fs::read_dir` | exit 1 | silent: `std::fs::read_dir` | exit 1 | not tagged: `std::fs::read_dir` | exit 1 |
| `std::fs::read_link` | `fs_read_link` names `std::fs::read_link` | exit 1 | silent: `std::fs::read_link` | exit 1 | not tagged: `std::fs::read_link` | exit 1 |
| `std::fs::read_to_string` | `fs_read_to_string` names `std::fs::read_to_string` | exit 1 | silent: `std::fs::read_to_string` | exit 1 | not tagged: `std::fs::read_to_string` | exit 1 |
| `std::fs::remove_dir` | `fs_remove_dir` names `std::fs::remove_dir` | exit 1 | silent: `std::fs::remove_dir` | exit 1 | not tagged: `std::fs::remove_dir` | exit 1 |
| `std::fs::remove_dir_all` | `fs_remove_dir_all` names `std::fs::remove_dir_all` | exit 1 | silent: `std::fs::remove_dir_all` | exit 1 | not tagged: `std::fs::remove_dir_all` | exit 1 |
| `std::fs::remove_file` | `fs_remove_file` names `std::fs::remove_file` | exit 1 | silent: `std::fs::remove_file` | exit 1 | not tagged: `std::fs::remove_file` | exit 1 |
| `std::fs::rename` | `fs_rename` names `std::fs::rename` | exit 1 | silent: `std::fs::rename` | exit 1 | not tagged: `std::fs::rename` | exit 1 |
| `std::fs::Permissions`<br>`std::fs::set_permissions` | `fs_set_permissions` names `std::fs::Permissions`, `std::fs::set_permissions` | exit 1 | silent: `std::fs::Permissions`, `std::fs::set_permissions` | exit 1 | not tagged: `std::fs::Permissions`, `std::fs::set_permissions` | exit 1 |
| `std::fs::soft_link` | `fs_soft_link` names `std::fs::soft_link` | exit 1 | silent: `std::fs::soft_link` | exit 1 | not tagged: `std::fs::soft_link` | exit 1 |
| `std::fs::symlink_metadata` | `fs_symlink_metadata` names `std::fs::symlink_metadata` | exit 1 | silent: `std::fs::symlink_metadata` | exit 1 | not tagged: `std::fs::symlink_metadata` | exit 1 |
| `std::fs::write` | `fs_write` names `std::fs::write` | exit 1 | silent: `std::fs::write` | exit 1 | not tagged: `std::fs::write` | exit 1 |
| `std::fs::DirBuilder` | `fs_dir_builder` names `std::fs::DirBuilder` | exit 1 | silent: `std::fs::DirBuilder` | exit 1 | not tagged: `std::fs::DirBuilder` | exit 1 |
| `std::fs::DirEntry` | `fs_dir_entry` names `std::fs::DirEntry` | exit 1 | silent: `std::fs::DirEntry` | exit 1 | not tagged: `std::fs::DirEntry` | exit 1 |
| `std::fs::File` | `fs_file` names `std::fs::File` | exit 1 | silent: `std::fs::File` | exit 1 | not tagged: `std::fs::File` | exit 1 |
| `std::fs::FileTimes` | `fs_file_times` names `std::fs::FileTimes` | exit 1 | silent: `std::fs::FileTimes` | exit 1 | not tagged: `std::fs::FileTimes` | exit 1 |
| `std::fs::FileType` | `fs_file_type` names `std::fs::FileType` | exit 1 | silent: `std::fs::FileType` | exit 1 | not tagged: `std::fs::FileType` | exit 1 |
| `std::fs::Metadata` | `fs_metadata_type` names `std::fs::Metadata` | exit 1 | silent: `std::fs::Metadata` | exit 1 | not tagged: `std::fs::Metadata` | exit 1 |
| `std::fs::OpenOptions` | `fs_open_options` names `std::fs::OpenOptions` | exit 1 | silent: `std::fs::OpenOptions` | exit 1 | not tagged: `std::fs::OpenOptions` | exit 1 |
| `std::fs::ReadDir` | `fs_read_dir_type` names `std::fs::ReadDir` | exit 1 | silent: `std::fs::ReadDir` | exit 1 | not tagged: `std::fs::ReadDir` | exit 1 |
| `std::fs::TryLockError` | `fs_try_lock_error` names `std::fs::TryLockError` | exit 1 | silent: `std::fs::TryLockError` | exit 1 | not tagged: `std::fs::TryLockError` | exit 1 |
| `std::path::Path::canonicalize` | `path_canonicalize` names `std::path::Path::canonicalize` | exit 1 | silent: `std::path::Path::canonicalize` | exit 1 | not tagged: `std::path::Path::canonicalize` | exit 1 |
| `std::path::Path::exists` | `path_exists` names `std::path::Path::exists` | exit 1 | silent: `std::path::Path::exists` | exit 1 | not tagged: `std::path::Path::exists` | exit 1 |
| `std::path::Path::is_dir` | `path_is_dir` names `std::path::Path::is_dir` | exit 1 | silent: `std::path::Path::is_dir` | exit 1 | not tagged: `std::path::Path::is_dir` | exit 1 |
| `std::path::Path::is_file` | `path_is_file` names `std::path::Path::is_file` | exit 1 | silent: `std::path::Path::is_file` | exit 1 | not tagged: `std::path::Path::is_file` | exit 1 |
| `std::path::Path::is_symlink` | `path_is_symlink` names `std::path::Path::is_symlink` | exit 1 | silent: `std::path::Path::is_symlink` | exit 1 | not tagged: `std::path::Path::is_symlink` | exit 1 |
| `std::path::Path::metadata` | `path_metadata` names `std::path::Path::metadata` | exit 1 | silent: `std::path::Path::metadata` | exit 1 | not tagged: `std::path::Path::metadata` | exit 1 |
| `std::path::Path::read_dir` | `path_read_dir` names `std::path::Path::read_dir` | exit 1 | silent: `std::path::Path::read_dir` | exit 1 | not tagged: `std::path::Path::read_dir` | exit 1 |
| `std::path::Path::read_link` | `path_read_link` names `std::path::Path::read_link` | exit 1 | silent: `std::path::Path::read_link` | exit 1 | not tagged: `std::path::Path::read_link` | exit 1 |
| `std::path::Path::symlink_metadata` | `path_symlink_metadata` names `std::path::Path::symlink_metadata` | exit 1 | silent: `std::path::Path::symlink_metadata` | exit 1 | not tagged: `std::path::Path::symlink_metadata` | exit 1 |
| `std::path::Path::try_exists` | `path_try_exists` names `std::path::Path::try_exists` | exit 1 | silent: `std::path::Path::try_exists` | exit 1 | not tagged: `std::path::Path::try_exists` | exit 1 |
| `std::os::unix::fs::chown` | `unix_chown` names `std::os::unix::fs::chown` | exit 1 | silent: `std::os::unix::fs::chown` | exit 1 | not tagged: `std::os::unix::fs::chown` | exit 1 |
| `std::os::unix::fs::chroot` | `unix_chroot` names `std::os::unix::fs::chroot` | exit 1 | silent: `std::os::unix::fs::chroot` | exit 1 | not tagged: `std::os::unix::fs::chroot` | exit 1 |
| `std::os::unix::fs::fchown` | `unix_fchown` names `std::os::unix::fs::fchown` | exit 1 | silent: `std::os::unix::fs::fchown` | exit 1 | not tagged: `std::os::unix::fs::fchown` | exit 1 |
| `std::os::unix::fs::lchown` | `unix_lchown` names `std::os::unix::fs::lchown` | exit 1 | silent: `std::os::unix::fs::lchown` | exit 1 | not tagged: `std::os::unix::fs::lchown` | exit 1 |
| `std::os::unix::fs::symlink` | `unix_symlink` names `std::os::unix::fs::symlink` | exit 1 | silent: `std::os::unix::fs::symlink` | exit 1 | not tagged: `std::os::unix::fs::symlink` | exit 1 |
| `std::env::args` | `env_args` names `std::env::args` | exit 1 | silent: `std::env::args` | exit 1 | not tagged: `std::env::args` | exit 1 |
| `std::env::args_os` | `env_args_os` names `std::env::args_os` | exit 1 | silent: `std::env::args_os` | exit 1 | not tagged: `std::env::args_os` | exit 1 |
| `std::env::current_dir` | `env_current_dir` names `std::env::current_dir` | exit 1 | silent: `std::env::current_dir` | exit 1 | not tagged: `std::env::current_dir` | exit 1 |
| `std::env::current_exe` | `env_current_exe` names `std::env::current_exe` | exit 1 | silent: `std::env::current_exe` | exit 1 | not tagged: `std::env::current_exe` | exit 1 |
| `std::env::home_dir` | `env_home_dir` names `std::env::home_dir` | exit 1 | silent: `std::env::home_dir` | exit 1 | not tagged: `std::env::home_dir` | exit 1 |
| `std::env::join_paths` | `env_join_paths` names `std::env::join_paths` | exit 1 | silent: `std::env::join_paths` | exit 1 | not tagged: `std::env::join_paths` | exit 1 |
| `std::env::remove_var` | `env_remove_var` names `std::env::remove_var` | exit 1 | silent: `std::env::remove_var` | exit 1 | not tagged: `std::env::remove_var` | exit 1 |
| `std::env::set_current_dir` | `env_set_current_dir` names `std::env::set_current_dir` | exit 1 | silent: `std::env::set_current_dir` | exit 1 | not tagged: `std::env::set_current_dir` | exit 1 |
| `std::env::set_var` | `env_set_var` names `std::env::set_var` | exit 1 | silent: `std::env::set_var` | exit 1 | not tagged: `std::env::set_var` | exit 1 |
| `std::env::split_paths` | `env_split_paths` names `std::env::split_paths` | exit 1 | silent: `std::env::split_paths` | exit 1 | not tagged: `std::env::split_paths` | exit 1 |
| `std::env::temp_dir` | `env_temp_dir` names `std::env::temp_dir` | exit 1 | silent: `std::env::temp_dir` | exit 1 | not tagged: `std::env::temp_dir` | exit 1 |
| `std::env::var` | `env_var` names `std::env::var` | exit 1 | silent: `std::env::var` | exit 1 | not tagged: `std::env::var` | exit 1 |
| `std::env::var_os` | `env_var_os` names `std::env::var_os` | exit 1 | silent: `std::env::var_os` | exit 1 | not tagged: `std::env::var_os` | exit 1 |
| `std::env::vars` | `env_vars` names `std::env::vars` | exit 1 | silent: `std::env::vars` | exit 1 | not tagged: `std::env::vars` | exit 1 |
| `std::env::vars_os` | `env_vars_os` names `std::env::vars_os` | exit 1 | silent: `std::env::vars_os` | exit 1 | not tagged: `std::env::vars_os` | exit 1 |
| `std::path::absolute` | `env_absolute` names `std::path::absolute` | exit 1 | silent: `std::path::absolute` | exit 1 | not tagged: `std::path::absolute` | exit 1 |
| `std::env::Args` | `env_args_type` names `std::env::Args` | exit 1 | silent: `std::env::Args` | exit 1 | not tagged: `std::env::Args` | exit 1 |
| `std::env::ArgsOs` | `env_args_os_type` names `std::env::ArgsOs` | exit 1 | silent: `std::env::ArgsOs` | exit 1 | not tagged: `std::env::ArgsOs` | exit 1 |
| `std::env::JoinPathsError` | `env_join_paths_error` names `std::env::JoinPathsError` | exit 1 | silent: `std::env::JoinPathsError` | exit 1 | not tagged: `std::env::JoinPathsError` | exit 1 |
| `std::env::SplitPaths` | `env_split_paths_type` names `std::env::SplitPaths` | exit 1 | silent: `std::env::SplitPaths` | exit 1 | not tagged: `std::env::SplitPaths` | exit 1 |
| `std::env::VarError` | `env_var_error` names `std::env::VarError` | exit 1 | silent: `std::env::VarError` | exit 1 | not tagged: `std::env::VarError` | exit 1 |
| `std::env::Vars` | `env_vars_type` names `std::env::Vars` | exit 1 | silent: `std::env::Vars` | exit 1 | not tagged: `std::env::Vars` | exit 1 |
| `std::env::VarsOs` | `env_vars_os_type` names `std::env::VarsOs` | exit 1 | silent: `std::env::VarsOs` | exit 1 | not tagged: `std::env::VarsOs` | exit 1 |
| `std::net::ToSocketAddrs`<br>`std::net::ToSocketAddrs::to_socket_addrs` | `net_to_socket_addrs` names `std::net::ToSocketAddrs`, `std::net::ToSocketAddrs::to_socket_addrs` | exit 1 | silent: `std::net::ToSocketAddrs`, `std::net::ToSocketAddrs::to_socket_addrs` | exit 1 | not tagged: `std::net::ToSocketAddrs`, `std::net::ToSocketAddrs::to_socket_addrs` | exit 1 |
| `std::net::AddrParseError` | `net_addr_parse_error` names `std::net::AddrParseError` | exit 1 | silent: `std::net::AddrParseError` | exit 1 | not tagged: `std::net::AddrParseError` | exit 1 |
| `std::net::Incoming` | `net_incoming` names `std::net::Incoming` | exit 1 | silent: `std::net::Incoming` | exit 1 | not tagged: `std::net::Incoming` | exit 1 |
| `std::net::IpAddr` | `net_ip_addr` names `std::net::IpAddr` | exit 1 | silent: `std::net::IpAddr` | exit 1 | not tagged: `std::net::IpAddr` | exit 1 |
| `std::net::Ipv4Addr` | `net_ipv4_addr` names `std::net::Ipv4Addr` | exit 1 | silent: `std::net::Ipv4Addr` | exit 1 | not tagged: `std::net::Ipv4Addr` | exit 1 |
| `std::net::Ipv6Addr` | `net_ipv6_addr` names `std::net::Ipv6Addr` | exit 1 | silent: `std::net::Ipv6Addr` | exit 1 | not tagged: `std::net::Ipv6Addr` | exit 1 |
| `std::net::Shutdown` | `net_shutdown` names `std::net::Shutdown` | exit 1 | silent: `std::net::Shutdown` | exit 1 | not tagged: `std::net::Shutdown` | exit 1 |
| `std::net::SocketAddr` | `net_socket_addr` names `std::net::SocketAddr` | exit 1 | silent: `std::net::SocketAddr` | exit 1 | not tagged: `std::net::SocketAddr` | exit 1 |
| `std::net::SocketAddrV4` | `net_socket_addr_v4` names `std::net::SocketAddrV4` | exit 1 | silent: `std::net::SocketAddrV4` | exit 1 | not tagged: `std::net::SocketAddrV4` | exit 1 |
| `std::net::SocketAddrV6` | `net_socket_addr_v6` names `std::net::SocketAddrV6` | exit 1 | silent: `std::net::SocketAddrV6` | exit 1 | not tagged: `std::net::SocketAddrV6` | exit 1 |
| `std::net::TcpListener` | `net_tcp_listener` names `std::net::TcpListener` | exit 1 | silent: `std::net::TcpListener` | exit 1 | not tagged: `std::net::TcpListener` | exit 1 |
| `std::net::TcpStream` | `net_tcp_stream` names `std::net::TcpStream` | exit 1 | silent: `std::net::TcpStream` | exit 1 | not tagged: `std::net::TcpStream` | exit 1 |
| `std::net::UdpSocket` | `net_udp_socket` names `std::net::UdpSocket` | exit 1 | silent: `std::net::UdpSocket` | exit 1 | not tagged: `std::net::UdpSocket` | exit 1 |
| `std::os::unix::net::UnixDatagram` | `unix_datagram` names `std::os::unix::net::UnixDatagram` | exit 1 | silent: `std::os::unix::net::UnixDatagram` | exit 1 | not tagged: `std::os::unix::net::UnixDatagram` | exit 1 |
| `std::os::unix::net::UnixListener` | `unix_listener` names `std::os::unix::net::UnixListener` | exit 1 | silent: `std::os::unix::net::UnixListener` | exit 1 | not tagged: `std::os::unix::net::UnixListener` | exit 1 |
| `std::os::unix::net::UnixStream` | `unix_stream` names `std::os::unix::net::UnixStream` | exit 1 | silent: `std::os::unix::net::UnixStream` | exit 1 | not tagged: `std::os::unix::net::UnixStream` | exit 1 |
| `std::process::abort` | `process_abort` names `std::process::abort` | exit 1 | silent: `std::process::abort` | exit 1 | not tagged: `std::process::abort` | exit 1 |
| `std::process::exit` | `process_exit` names `std::process::exit` | exit 1 | silent: `std::process::exit` | exit 1 | not tagged: `std::process::exit` | exit 1 |
| `std::process::id` | `process_id` names `std::process::id` | exit 1 | silent: `std::process::id` | exit 1 | not tagged: `std::process::id` | exit 1 |
| `std::process::Child` | `process_child` names `std::process::Child` | exit 1 | silent: `std::process::Child` | exit 1 | not tagged: `std::process::Child` | exit 1 |
| `std::process::ChildStderr` | `process_child_stderr` names `std::process::ChildStderr` | exit 1 | silent: `std::process::ChildStderr` | exit 1 | not tagged: `std::process::ChildStderr` | exit 1 |
| `std::process::ChildStdin` | `process_child_stdin` names `std::process::ChildStdin` | exit 1 | silent: `std::process::ChildStdin` | exit 1 | not tagged: `std::process::ChildStdin` | exit 1 |
| `std::process::ChildStdout` | `process_child_stdout` names `std::process::ChildStdout` | exit 1 | silent: `std::process::ChildStdout` | exit 1 | not tagged: `std::process::ChildStdout` | exit 1 |
| `std::process::Command` | `process_command` names `std::process::Command` | exit 1 | silent: `std::process::Command` | exit 1 | not tagged: `std::process::Command` | exit 1 |
| `std::process::CommandArgs` | `process_command_args` names `std::process::CommandArgs` | exit 1 | silent: `std::process::CommandArgs` | exit 1 | not tagged: `std::process::CommandArgs` | exit 1 |
| `std::process::CommandEnvs` | `process_command_envs` names `std::process::CommandEnvs` | exit 1 | silent: `std::process::CommandEnvs` | exit 1 | not tagged: `std::process::CommandEnvs` | exit 1 |
| `std::process::ExitCode` | `process_exit_code` names `std::process::ExitCode` | exit 1 | silent: `std::process::ExitCode` | exit 1 | not tagged: `std::process::ExitCode` | exit 1 |
| `std::process::ExitStatus` | `process_exit_status` names `std::process::ExitStatus` | exit 1 | silent: `std::process::ExitStatus` | exit 1 | not tagged: `std::process::ExitStatus` | exit 1 |
| `std::process::Output` | `process_output` names `std::process::Output` | exit 1 | silent: `std::process::Output` | exit 1 | not tagged: `std::process::Output` | exit 1 |
| `std::process::Stdio` | `process_stdio` names `std::process::Stdio` | exit 1 | silent: `std::process::Stdio` | exit 1 | not tagged: `std::process::Stdio` | exit 1 |
| `std::process::Termination` | `process_termination` names `std::process::Termination` | exit 1 | silent: `std::process::Termination` | exit 1 | not tagged: `std::process::Termination` | exit 1 |

### Verification: `1169dca`

```text
$ just effects
sh scripts/check-effects.sh
effects: libpachinko names none of 8 banned crates and uses none of 108 banned std APIs;
effects: the fixture trips every ban exactly once, and pachinko trips none.
Exit status: 0

$ just metrics
::notice::Quality score: 100.0% (2 functions analyzed)
::notice::Quality score: 100.0% (4 functions analyzed)
Exit status: 0

$ rm -rf target/effects   # a cold build cache for gate 8
Exit status: 0
$ sandbox-exec -p "$deny" curl -fsS --max-time 10 https://static.rust-lang.org
curl: (6) Could not resolve host: static.rust-lang.org
Exit status: 6
$ sandbox-exec -p "$deny" curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1
curl: (7) Failed to connect to 1.1.1.1 port 443 after 1 ms: Couldn't connect to server
Exit status: 7
$ sandbox-exec -p "$deny" just effects
sh scripts/check-effects.sh
effects: libpachinko names none of 8 banned crates and uses none of 108 banned std APIs;
effects: the fixture trips every ban exactly once, and pachinko trips none.
Exit status: 0

$ cargo tree --manifest-path tests/fixtures/effects-violation/Cargo.toml -e normal --prefix none --all-features --target all --locked --offline
effects-violation v0.1.0 (tests/fixtures/effects-violation)
async-std v0.1.0 (tests/fixtures/effects-violation/deps/async-std)
getrandom v0.1.0 (tests/fixtures/effects-violation/deps/getrandom)
mio v0.1.0 (tests/fixtures/effects-violation/deps/mio)
rand v0.1.0 (tests/fixtures/effects-violation/deps/rand)
rand_core v0.1.0 (tests/fixtures/effects-violation/deps/rand_core)
smol v0.1.0 (tests/fixtures/effects-violation/deps/smol)
tokio v0.1.0 (tests/fixtures/effects-violation/deps/tokio)
wrapper v0.1.0 (tests/fixtures/effects-violation/deps/wrapper)
socket2 v0.1.0 (tests/fixtures/effects-violation/deps/socket2)
Exit status: 0
```

§12 claim 11 holds: rustqual 1.8.3 reports nothing on either crate. §12 claim 12
holds: gate 8 runs with the network denied at the OS level and a cold build cache
(`target/effects` removed first), with both negative controls failing. A machine
with no toolchain still needs `rustup toolchain install`, which §11 names as a
network step outside `check`.

### Checkpoint 2 review (§14): the lane's diff and step 7's red runs

Same reviewers and triage as checkpoint 1. The brief held the ticket, CONVENTIONS,
`git diff main...HEAD` at `51303ae` and the red-run artefacts. Raw files are under
`ai_tmp/review/T10-2/`.

| Id | Finding | Verdict | Applied |
| --- | --- | --- | --- |
| T10-C1 (medium) | Deleting a ban from `clippy.toml` or the script removes it from the probe's expectation too, so it goes silent | valid: reproduced, gate green at 107 bans | the fixture's `// ban:` tags and its stubs became an inventory the lists are held to (`1169dca`) |
| T10-C2 (medium) | Hand-back evidence absent | invalid at review time: written after the review, by design | these notes |
| T10-C3 (low) | `unsafe` blocks in the fixture break invariant 4 | valid | `set_var` and `remove_var` taken as values; `#![forbid(unsafe_code)]` in the fixture |
| B1 (high) | Only `--all-features` on the host target: `cfg(not(feature))`, another target's dependencies and target-gated code go unseen | valid, rated medium: latent, since libpachinko has no features or `cfg` | `cargo tree --target all`; a second clippy pass with `--no-default-features`; `--no-deps` on the pachinko run; target-gated std code handed back |
| B2 (medium) | As T10-C1 | valid, duplicate | as T10-C1 |
| B3 (medium) | The red harness counts any non-zero exit as a pass | invalid: every recorded result names its own entries | tightened anyway: an `OK` now needs the exact named set |
| B4 (medium) | Network-denial evidence absent | invalid: re-run cold under `sandbox-exec`, green, both controls failing | quoted above |

No high finding was valid as high, so no second round ran. The fixes landed in
`1169dca`, and step 7 and the verification were re-run there (quoted above).

### Final gate: `1169dca` with uncommitted changes (these notes)

```text
$ just check
==> just check-toolchain
==> just lock-check
==> just lint
==> just fmt-check
==> just toml-check
==> just clippy
==> just metrics
==> just effects
==> just features
==> just test-doc
==> just coverage
==> just doc
==> just deny
==> just deps-unused
==> just check-docs
==> just check-agents
==> just check-specs
==> just analyse-specs
==> just snapshots-check
==> just check-clean
...
effects: libpachinko names none of 8 banned crates and uses none of 108 banned std APIs;
effects: the fixture trips every ban exactly once, and pachinko trips none.
...
The worktree matches the check baseline.

All checks passed and the worktree is unchanged.
Exit status: 0
```

### For T11

- `docs/explanation/architecture.md`: libpachinko's effects are banned mechanically
  by gate 8, `just effects`. Its crate bans are checked against libpachinko's normal
  dependency tree on every target. Its std-API bans are clippy's
  `disallowed-methods` and `disallowed-types` in `crates/libpachinko/clippy.toml`,
  which clippy reads for libpachinko alone, so the service keeps every effect.
  Clippy resolves names, so an alias, a glob, a macro body, a function value and a
  method on a value passed in are all caught.
- `docs/reference/quality-gates.md`: gate 8 runs `scripts/check-effects.sh`, offline,
  in four checks. (0) `crates/libpachinko/clippy.toml` still restates the root
  `clippy.toml`. (1) libpachinko names no banned crate, and uses no banned std API
  with every feature or with none. (2) `tests/fixtures/effects-violation/` trips
  every ban exactly once, and its stubs and `// ban:` tags name exactly the ban lists.
  (3) pachinko trips none.
- `docs/reference/configuration.md`: `crates/libpachinko/clippy.toml` shadows the
  root `clippy.toml` instead of merging with it, so it restates every root line
  verbatim, and gate 8 fails when they drift. A root change is mirrored there. A ban
  is added to that file with a reason beginning `effect:`, and broken once in the
  fixture with a `// ban:` tag. A path that does not resolve is only a warning, so
  the fixture is what proves an entry is live.
- Decision 0008: rustqual's `forbid_path_prefix` was rejected for the std-API bans.
  It matches text: it missed `use std as s`, `use std::*`, a macro body and a
  renamed import, and it could ban `elapsed` only by name, on every type. Clippy
  needs the bans as item paths, so the file lists 108 of them, built from the
  `#[stable]` items of std. A new std item in a banned module is not banned until
  it is listed.

## Open points

- `std::process` is banned with the rest: a library has no business spawning. If a
  specification ever needs it, it becomes an effect trait like the others.
- Whether `std::sync` primitives other than thread spawning (`Mutex`, atomics) are
  allowed. They are not effects and stay allowed unless the maintainer says otherwise.
- **`std::env::consts`.** §9 bans "everything under `std::env`", but clippy cannot ban
  a constant, so `std::env::consts::{OS,ARCH,FAMILY,...}` stay reachable. They are
  compile-time values, like `cfg!(target_os)` and `env!`, not effects. The
  maintainer decides whether §9 means them. If it does, the smallest fix is one
  rustqual `forbid_path_prefix = ["std::env::consts"]` rule in `rustqual-effects.toml`,
  run by the script over libpachinko.
- **Target-gated std code.** Clippy lints only what the host compiles, so a banned
  call under `#[cfg(windows)]` in libpachinko passes on the macOS and Linux gates.
  Covering it means `rustup target add`, a network step, plus a clippy pass per
  target. Dependencies are covered on every target already.
- **New std items.** The lists are item paths, so a function added to `std::fs` in a
  later Rust is not banned until it is listed. The toolchain bump (§13) is the point
  to re-run the inventory.
- **Feature combinations.** Two passes, every feature and none, cover a
  `cfg(feature)` and a `cfg(not(feature))`, not a mix of mutually exclusive features.
- **Mtimes.** A revert that restores an older mtime (`rsync -a`, `cp -p`) can let
  cargo replay stale clippy warnings, in gate 8 and in gate 6 alike.
