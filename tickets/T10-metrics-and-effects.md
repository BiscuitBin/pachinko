---
id: T10
title: "Metrics and the effects boundary: rustqual over both crates, the libpachinko bans, the probe"
status: open
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

To be written by the executing agent: the evidence for `CONVENTIONS.md` §12 claims 2, 11
and 12; the mechanism chosen and why; each red run; the review record for each checkpoint
(`CONVENTIONS.md` §14); the sentences T11 must carry into
`docs/explanation/architecture.md`, `docs/reference/quality-gates.md`,
`docs/reference/configuration.md` and decision 0008.

## Open points

- `std::process` is banned with the rest: a library has no business spawning. If a
  specification ever needs it, it becomes an effect trait like the others.
- Whether `std::sync` primitives other than thread spawning (`Mutex`, atomics) are
  allowed. They are not effects and stay allowed unless the maintainer says otherwise.
