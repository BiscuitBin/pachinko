# Conventions for bootstrapping `pachinko`

This document is the design every ticket under `tickets/` obeys. A ticket cites it by
section (`CONVENTIONS.md §4`) instead of restating it, and embeds exact content only where
the agent executing the ticket would otherwise have to guess. Where a ticket and this
document disagree, this document wins and the ticket is corrected. Changes to this
document go through a pull request on `main`, never through a lane branch, because every
lane reads it.

Every ticket is done. On 2026-10-24 this document was corrected to match what landed, so
a fact stated here is true of the tree; the tickets' hand-back notes record how each one
came about.

The foundation is a copy of libpawdoku's, adapted. Almost every file a ticket writes
already exists, finished and green, in L at the pinned commit; a ticket names that file and
the delta, and the agent reads the original. Restating L here would only give the two
copies room to disagree.

## 0. What is being built, and where the sources are

`BiscuitBin/pachinko` (public, on the free organisation plan) began as one `first commit`
(`9ed604c`) holding a stub `README.md`, plus this `tickets/` directory on
`bootstrap-tickets`, merged as pull request #1 (`fca730e`). It became a Cargo workspace of
two crates:

- **`crates/libpachinko`**, crate name `libpachinko`, a std library. It holds the domain
  once the Allium specifications define one, and it never performs an effect itself (§1,
  decision 3).
- **`crates/pachinko`**, crate name `pachinko`, a minimal std binary built around the
  library. It supplies the real implementations of the library's effect traits.

Both are Apache-2.0 with `publish = false`. The domain is undefined: no ticket here writes
domain code, domain prose or a behavioural specification. The repository carries the
toolchain, gates, handbook, agent contract, CI and Allium tooling L carries, so that the
specifications and the code land in a repository that is already green.

Three repositories are the sources. This document and every ticket cite them with a
letter, a code-span path and, where it matters, line numbers at these exact commits:

| Letter | Repository | Local clone | Commit |
| --- | --- | --- | --- |
| **L** | `steven-cutting/libpawdoku`, the Rust sudoku engine; the source of almost everything | `/Users/scutting/projects/libpawdoku` | `ccdd001` |
| **B** | `steven-cutting/biscuit_games_tooling`, the `bg-*` checkers and the Allium installer | `/Users/scutting/projects/biscuit_games_tooling` | `6c5c07f` (= tag `v0.3.0`) |
| **T** | `steven-cutting/biscuit_games_template`, the Copier template; its `scripts/bootstrap_repo.sh` applies repository settings | `/Users/scutting/projects/biscuit_games_template` | `2283589c` |

Read a pinned file with `git -C <clone> show <commit>:<path>`; never modify a clone. A
citation in a ticket is written `L` followed by the path as a code span, for example L
`Justfile` or L `tickets/S03-bench-fuzz-mutants.md` lines 553-687, so that it can be
checked mechanically.

L's own tickets are the second source. Three of its spikes drafted, but never built, the
work wave 2 did here, and the wave-2 tickets adapted those drafts rather than re-deriving
them:

| Pachinko ticket | L draft | Where |
| --- | --- | --- |
| T14 release | L's T13 | L `tickets/S02-release-and-publishing.md` lines 648-866 |
| T15 mutation testing | L's T14 | L `tickets/S03-bench-fuzz-mutants.md` lines 553-687 |
| T16 benchmarks and fuzzing | the two untitled drafts | L `tickets/S03-bench-fuzz-mutants.md` lines 688-911 |
| T17 Renovate | L's T18 | L `tickets/S01-dependency-updates.md` lines 666-905 |

Each spike's "What was verified, and how" section, above the draft, holds the evidence
the draft rests on. A wave-2 ticket reads it, not just the draft.

## 1. Decisions taken with the maintainer, and the facts everything rests on

Decisions, taken on 2026-10-21:

1. **The domain is left undefined.** The foundation stays domain-neutral, and the Allium
   specifications define the domain later. No ticket names a domain noun.
2. **libpachinko is a std library.** No `#![no_std]`, no wasm targets, no `wasm-check`.
3. **Effects still sit behind library-owned traits, with fakes beside them.** libpachinko
   never touches the clock, randomness, the filesystem, the network, the environment or
   threads directly; the `pachinko` service supplies the real implementations. This is
   enforced mechanically, not by convention (§9). The traits themselves arrive with the
   specifications that name the effects.
4. **The service is a minimal std binary** that depends on libpachinko. Its `main` is one
   line calling a tested `run()`. The async runtime, transport and HTTP stack are deferred
   to a spike and a decision record after the specifications (decision 0014).
5. **Apache-2.0 for both crates, with `publish = false`.**
6. **Tickets live in `tickets/`**, in L's format.
7. **Everything is carried over:** pixi, prek and B; the handbook and decision records;
   the agent contract and skills, including the seven Allium skills and
   `allium-skill-reference/`; the full Rust gate and CI; all the Allium tooling.
8. **Four extras are in the foundation, not deferred:** a container image built in CI,
   Renovate, release tooling, and mutation testing with benchmarks and fuzzing (wave 2).

Facts, each verified in source on 2026-10-21, that shape the mechanism:

1. **`docs/specs/` cannot be empty.** B `src/biscuit_games_tooling/run_allium.py` line 131
   raises `RuntimeError` when allium resolves no module, so `check-specs` and
   `analyse-specs` need one. T00 ships a placeholder root module, `docs/specs/pachinko.allium`,
   that declares no behaviour (§9).
2. **`AGENTS.md` must still contain the word `runes`.** B
   `src/biscuit_games_tooling/validate_agents.py` lines 47-54 require six phrases:
   `untrusted`, `just check`, `explicit authorization`, `ai_tmp/`, `docs/specs/` and
   `runes`. `AGENTS.md` keeps L's honest sentence about runes (L `AGENTS.md`, Provenance).
3. **cargo-deny's `wrappers` names direct parents only.** `wrappers = ["pachinko"]` lets
   `pachinko` name `tokio` itself, but a `tokio` that arrives through another crate
   `pachinko` depends on is still denied, and a banned crate that arrives in libpachinko
   through an allowed one is caught only if the ban lists it. `wrappers` cannot say
   "anything under `pachinko`", so a workspace-wide ban would reject the service's own
   transitives (`pachinko -> axum -> tokio`). `deny.toml` therefore carries no
   effect-crate bans; the only crate check walks libpachinko's normal dependency tree
   (§9).
4. **clippy's `disallowed-methods` and `disallowed-types` take item paths, not modules.**
   Banning `std::fs` through clippy means listing `std::fs::File`, `std::fs::read` and the
   rest; rustqual's `forbid_path_prefix` (L `rustqual.toml`, the `[[architecture.pattern]]`
   tables) takes a module prefix. The mechanism was T10's decision (§9, §12), and T10
   chose clippy: `disallowed-methods` and `disallowed-types` in
   `crates/libpachinko/clippy.toml`, 121 entries whose reasons begin `effect:`, beside
   the `cargo tree` crate line in `scripts/check-effects.sh` and the probe over
   `tests/fixtures/effects-violation/`.
5. **L's live state is the baseline, not L's T00.** L's gate grew after its foundation:
   `metrics` (rustqual, L's T22), `snapshots-check` (insta, L's T29), `plan-spec` (L's
   T24), `site` and `pages.yml` (L's T33). Pachinko's T00 starts from L's files at
   `ccdd001`, so all of these exist from the first commit.
6. **No runtime dependency until the specifications.** cargo-shear (gate `deps-unused`)
   flags a dependency nothing uses, so L's thiserror, serde, proptest and insta are not
   copied. The insta recipes land; the `insta` crate does not until something is
   snapshotted. Wave 1 added no dependency. Wave 2's harnesses added two, each with its
   decision-0007 note (`docs/decisions/0007-dependency-policy.md`, "Harness
   dependencies"): T16's divan, through `[workspace.dependencies]`, as libpachinko's only
   development dependency, and libfuzzer-sys in the excluded `fuzz/` crate, which
   `fuzz/Cargo.lock` pins.
7. **B's pixi git dependency is public.** `steven-cutting/biscuit_games_tooling` stays the
   URL in `pyproject.toml`; only this repository's own URLs move to `BiscuitBin/pachinko`.

## 2. Toolchain and pins

### `rust-toolchain.toml` (T00 creates it; frozen)

```toml
[toolchain]
# An exact stable release, bumped deliberately. `channel = "stable"` would move the
# clippy lint set every six weeks and break `-D warnings` with no diff. rust-version
# in Cargo.toml follows this pin until a crate is first published.
channel = "1.99.1"
profile = "minimal"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
```

The channel was the newest stable release on the day T00 executed, read from
`https://static.rust-lang.org/dist/channel-rust-stable.toml` (the `[pkg.rust] version`
line). On 2026-10-21 that was `1.99.1`, the pin today; L pins `1.98.1`. `rust-version` in
`[workspace.package]` is the pin's `major.minor`. Against L: no `targets` line (decision
2), and no `rust-src` (it served rust-analyzer only; a contributor adds it with
`rustup component add`). `llvm-tools-preview` serves cargo-llvm-cov. Renovate's
`rust toolchain` group now proposes each bump and holds it for dashboard approval
(decision 0018).

Because L's lint table was tuned against 1.98.1, T00 runs `just clippy` on the new pin
before freezing anything, and T02 owns any lint the new stable added or renamed (§12
claim 3).

### `pyproject.toml` (the pixi manifest; T00 writes it with its final comments; frozen)

L `pyproject.toml` with these changes and no others:

- `[project] name = "pachinko-tooling"`, and the description names pachinko.
- The `recipes` list in `[tool.biscuit-games-tooling]` is §4's gate order.
- The comments describe this repository, not L.
- Every pin stays as L has it (`just ==1.58.0`, `prek ==0.5.3`, `cargo-binstall ==1.23.0`,
  `cargo-nextest ==0.9.146`, `cargo-llvm-cov ==0.9.1`, `cargo-deny ==0.20.2`,
  `cargo-insta ==1.48.0`, `cargo-shear ==1.13.4`, `taplo ==0.10.0`, Python `3.14.*`, B at
  `v0.3.0`). Moving them is Renovate's job (T17), not the foundation's.

Wave 2 added two pins of its own, `cargo-semver-checks ==0.51.0` (T14) and
`cargo-mutants ==27.1.0` (T15). Renovate reads these pins and holds each bump it
proposes for dashboard approval; a pixi entry stays a notice until the hosted app is
shown to relock `pixi.lock` (decision 0018).

`pixi.lock` is generated by `pixi lock`, committed, and never hand-edited. Pixi owns the
tools and the Python environment; rustup owns the compiler; the environment never gains
`rust` (L `docs/decisions/0011-tool-manager.md`).

### `tools.txt` and `tools-source.txt` (T00 creates them; frozen in wave 1)

L `tools.txt` (`cargo-hack@0.6.45`) and L `tools-source.txt` (`rustqual@1.8.3`), each
with its comment block, verbatim, except that a decision cross-reference follows §8's
renumbering: `tools-source.txt` cites decision 0012, the metrics gate, where L cites its
0013 (an implied delta, recorded in T00's Deviations). Wave-2 tickets added their own
lines (§11): T14's `cargo-release@1.1.7` and T16's `cargo-fuzz@0.13.2` in `tools.txt`. On
2026-10-24 `tools-source.txt`'s "Bumped by hand" sentence became "Renovate proposes each
bump and holds it for dashboard approval (decision 0018)".

### Maintainer prerequisites (not ticket steps)

As L `tickets/CONVENTIONS.md` §2 "Maintainer prerequisites": rustup installed and its
`~/.cargo/bin/cargo` first on `PATH`; pixi 0.81.0 or newer; any just; gh authenticated
with access to `BiscuitBin`. If an agent's shell finds `~/.pixi/bin/cargo` first, every
command is prefixed `PATH="$HOME/.cargo/bin:$PATH"`. docker is a prerequisite for
`just image` only (T13, §12 claim 9). Since T16, `just check` also needs a C++ compiler,
because libfuzzer-sys compiles C++ when `just clippy` builds the fuzz crate, and
`just fuzz` on Apple silicon needs Rosetta 2 (`docs/how-to/develop-locally.md`).

## 3. Target repository tree (exact paths and owners)

Owner is the lane that writes the final content. T00 creates every path below that has
T00 in its row, as a stub or in final form, so that `just check` is green on the first
commit. A path that a lane creates is marked `new`; no other lane touches it. No path has
two wave-1 owners. `(frozen)` marks files no wave-1 lane touches after T00 (§11); the
freeze lifted after T11.

```text
Cargo.toml                          T00 skeleton -> T02 (lints, profiles)
Cargo.lock                          T00 (generated; two members, no dependencies) -> T16
rust-toolchain.toml                 T00 (frozen)
rustfmt.toml  clippy.toml           T00 working -> T02
taplo.toml  deny.toml               T00 working -> T02
.config/nextest.toml                T00 working -> T02
rustqual.toml                       T00 working -> T10
tools.txt  tools-source.txt         T00 (frozen in wave 1)
crates/libpachinko/Cargo.toml       T00 -> T02
crates/libpachinko/src/lib.rs       T00 (one documented, tested item) -> T02
crates/libpachinko/README.md        T00 stub -> T12
crates/pachinko/Cargo.toml          T00 -> T02
crates/pachinko/src/lib.rs          T00 (the tested run()) -> T02
crates/pachinko/src/main.rs         T00 (frozen; the one-line main)
crates/pachinko/tests/binary.rs     T00 (runs the built binary) -> T02
crates/pachinko/README.md           T00 stub -> T12
scripts/check-effects.sh            T00 working -> T10
tests/fixtures/effects-violation/   T10 new
crates/libpachinko/clippy.toml      T10 new (T10 chose clippy, §9)
LICENSE                             T00 (Apache-2.0 text, final)
README.md  CHANGELOG.md  SECURITY.md   T00 stub -> T12
AGENTS.md                           T00 stub (six phrases, 300 words) -> T05
CLAUDE.md  .github/copilot-instructions.md   T00 (byte-identical to L; frozen)
.agents/skills/<14>/SKILL.md        T00 stub -> T05
.claude/skills/<14>/SKILL.md  .codex/skills/<14>/SKILL.md   T00 (fixed bridge body; frozen)
.claude/settings.json               T00 (byte-identical to L; frozen)
allium-skill-reference/             T05 new (no `skills-lock.json`, §7)
Justfile                            T00 (frozen in wave 1)
.pre-commit-config.yaml  .pre-commit-fix.yaml   T00 (frozen)
.editorconfig  .gitattributes  .gitignore   T00 (frozen)
.markdownlint-cli2.jsonc  lychee.toml  _typos.toml   T00 (frozen)
pyproject.toml                      T00 (frozen)
pixi.lock                           T00 (generated; frozen in wave 1)
scripts/initialize.sh               T00 (frozen)
.github/actions/setup/action.yml    T00 stub -> T01
.github/workflows/ci.yml            T00 one-job stub -> T01
.github/workflows/audit.yml  .github/workflows/pages.yml   T01 new
docs/manifest.yml  docs/README.md   T00 (frozen in wave 1)
docs/project/*.md                   T00 stub -> T07
docs/tutorials/first-change.md      T00 stub -> T07
docs/how-to/{develop-locally,test-and-debug,maintain-dependencies,deploy-to-github-pages}.md   T00 stub -> T07
docs/how-to/work-with-the-specs.md  T00 stub -> T06
docs/explanation/specifications.md  T00 stub -> T06
docs/explanation/{architecture,layering,quality-philosophy,security-model}.md   T00 stub -> T07
docs/reference/*.md  docs/operations/*.md   T00 stub -> T07
docs/decisions/README.md  docs/decisions/0001..0014-*.md   T00 stub -> T07
docs/specs/pachinko.allium          T00 placeholder -> T06
tickets/                            this directory
ai_tmp/  .tools/  .pixi/  target/   gitignored; never committed
```

Not carried from L: `crates/pawdoku/**`, every `docs/specs/*.allium` (the nine sudoku
modules), `docs/explanation/{human-solving,solving-sudoku,puzzle-design}.md`, L's decisions
0008 (no_std), 0012, 0014 and 0015, `tests/fixtures/metrics-violation/` (its rules were
sudoku layering), and every snapshot.

`rustqual-effects.toml`, the rustqual alternative §9 allowed, was never created. Wave 2
created paths of its own, each listed in its ticket: `Dockerfile`, `.dockerignore` and
`docs/how-to/build-the-container-image.md` (T13); `.github/workflows/release.yml` (T14);
`.github/workflows/mutants.yml` (T15); `crates/libpachinko/benches/`, the excluded `fuzz/`
crate with its own `Cargo.lock`, the third lockfile, and `.github/workflows/fuzz.yml`
(T16); `renovate.json` (T17); and decisions 0015 to 0018 (§8).

### The two crates (T00 writes them; T02 finishes them)

Root `Cargo.toml` is L `Cargo.toml` with: `repository` and `homepage` set to
`https://github.com/BiscuitBin/pachinko`; `rust-version` per §2; no
`[workspace.dependencies]`; the `std_instead_of_core`, `std_instead_of_alloc` and
`alloc_instead_of_core` lines removed (decision 2); the `module_name_repetitions` line
removed (T02: the lint is in the restriction group, which the table never enables); the
`[profile.release]` comment about pyo3 removed. T00 lands the full lint table at once,
because `-D warnings` is decided by it and a skeleton with fewer lints would only move
the failures into T02. Wave 2 later added `[workspace.metadata.release]` (T14), and
`exclude = ["fuzz"]` with a `[workspace.dependencies]` table holding divan (T16).

`crates/libpachinko/Cargo.toml`: L `crates/pawdoku/Cargo.toml`'s package shape, every
field `.workspace = true`, `name = "libpachinko"`, a domain-neutral `description`
with `keywords`, `categories` and `readme` (the metadata the `cargo` group's
`cargo_common_metadata` names; the lint skips a `publish = false` package, so they are
kept for the docs and for the day publishing is decided), no `no-std` category, no
`[features]` and no dependencies. `src/lib.rs`: the crate docs (stating that the domain
arrives with the specifications) and one item, `pub const fn version() -> &'static str`
returning `env!("CARGO_PKG_VERSION")`, with a doctest and an in-module unit test, so
that every gate has something to format, lint, test, document and measure. `env!` is
read at compile time and is not the effect decision 3 bans. Wave 2 added
`[package.metadata.release]` (T14), and the divan development dependency with the
`version` bench under `benches/` (T16); libpachinko still has no normal dependency.

`crates/pachinko/Cargo.toml`: the same shape, `name = "pachinko"`, one dependency,
`libpachinko = { path = "../libpachinko", version = "0.1.0" }`. Cargo then enforces the
direction: `libpachinko` cannot name `pachinko` without a cycle; T14 later added its
`[package.metadata.release]`. The package has a library target and a binary target:

- `src/lib.rs`: `pub fn run(out: &mut impl std::io::Write) -> std::io::Result<()>`,
  which writes one line, `pachinko <libpachinko::version()>`. Taking the writer keeps
  `print_stdout` quiet and makes `run` testable in-process; its in-module test writes
  to a `Vec<u8>`.
- `src/main.rs`: one line of behaviour, calling `pachinko::run` with
  `std::io::stdout().lock()` and turning the result into a `std::process::ExitCode`.
- `tests/binary.rs`: runs `env!("CARGO_BIN_EXE_pachinko")` and asserts the line and the
  exit status. cargo-llvm-cov instruments a binary an integration test runs, so `main`
  is covered (§12 claim 4).

## 4. `Justfile` (T00 writes it; frozen in wave 1)

L `Justfile` at `ccdd001`, with exactly these changes at T00. A decision cross-reference
follows §8's renumbering, an implied delta: the `install-tools` comment cites decision
0012, the metrics gate, where L cites its 0013 (T00's Deviations). The later changes are
listed after the gate order.

- The `coverage_floor` comment reads "over crates/*/src/**" (still `"90"`).
- `wasm-check` is deleted, with its comment.
- `metrics` keeps L's version check and its first rustqual call, run once per crate:
  `rustqual crates/libpachinko …` and `rustqual crates/pachinko …`, each with
  `--config rustqual.toml --fail-on-warnings --format github`. L's probe block (the
  `status`, `out`, `want`, `got` lines and the comment sentences about the fixture) is
  removed: L's pattern rules were sudoku layering and are not carried, so there is no
  rule to probe. The comment says the effect bans and their probe are gate 8.
- A new recipe after `metrics`, with this text:

  ```just
  # Gate 8: libpachinko performs no effect itself (decision 0008). The script
  # checks libpachinko's dependency tree and source against the bans, then runs
  # the same checks over tests/fixtures/effects-violation, which breaks each
  # ban once, and fails unless every ban fires there and none fires on the
  # pachinko crate. The crate bans and the mechanism are the script's, the
  # std-API bans the `effect:` entries in crates/libpachinko/clippy.toml; this
  # recipe only runs it.
  effects:
      sh scripts/check-effects.sh
  ```

- `site` writes a root `index.html` that links to both crates' pages
  (`libpachinko/index.html` and `pachinko/index.html`) instead of redirecting to one, with
  the title "pachinko: API reference".
- No other recipe changed at T00. `snapshots-check`, `snapshots-review`,
  `snapshots-accept`, `plan-spec`, `install-tools`'s two passes, `audit`,
  `check-links-online` and `check-clean` are L's text.

`coverage` is not changed to ignore `main.rs`. If T00 finds that the binary's lines are
not counted (§12 claim 4), T00 adds `--ignore-filename-regex 'crates/pachinko/src/main\.rs'`
to the `coverage` recipe's `report` lines before the Justfile freezes, and records why.
It did not need to: `main.rs` is counted (§12 claim 4).

**Gate order** (the `recipes` list in `pyproject.toml`), then `check-clean`:

1. `check-toolchain`
2. `lock-check`
3. `lint`
4. `fmt-check`
5. `toml-check`
6. `clippy`
7. `metrics`
8. `effects`
9. `features`
10. `test-doc`
11. `coverage`
12. `doc`
13. `deny`
14. `deps-unused`
15. `check-docs`
16. `check-agents`
17. `check-specs`
18. `analyse-specs`
19. `snapshots-check`

`test`, `build`, `audit`, `site`, `plan-spec` and `check-links-online` are outside
`check`, as in L. Wave 2 added recipes outside `check` only (§11): `semver` (T14),
`mutants` (T15), `image` (T13), and `bench` and `fuzz` (T16).

Since T00 the `Justfile` has also changed in these ways:

- T16 extended `sync`, `lock`, `lock-upgrade`, `lock-check`, `format`, `fix`, `fmt-check`
  and `clippy` to the excluded `fuzz/` crate with `--manifest-path fuzz/Cargo.toml`, so
  `lock-check` checks three lockfiles. Clippy over `fuzz/` stays in `just check` by the
  maintainer's choice of 2026-10-24.
- On 2026-10-24 `check-toolchain` began refusing a toolchain that is not the pin, or that
  `RUSTUP_TOOLCHAIN` or a rustup override selected, and not only a cargo that is not
  rustup's proxy; `install-toolchain`'s comment lost "and targets", since no target is
  listed; and the `effects` comment above began naming clippy's file as the home of the
  std-API bans.

## 5. Hooks and dotfiles

`.pre-commit-config.yaml` and `.pre-commit-fix.yaml` are L's at `ccdd001`, byte for byte
(neither names a crate; their `files:` patterns already read `crates/.*`). Every remote
hook keeps L's rev and comment. One delta: the `typos` hook gains
`args: [--force-exclude]`, so the read-only gate never rewrites a file (upstream's
default adds `--write-changes`). Both were frozen at T00. Two deltas came later: T16's
`fmt-check` hook also matches `fuzz/`, and on 2026-10-24 the fix config's markdownlint
hook gained `pass_filenames: false`, because its config globs were appended to every
parallel batch prek passed it, and the batches raced `--fix` over the same files.

`.editorconfig`, `.gitattributes`, `.gitignore`, `.markdownlint-cli2.jsonc`,
`lychee.toml`, `_typos.toml` and `taplo.toml` are L's at `ccdd001`. T00 copies them, then
removes every line that serves a file type or tool this repository does not have, or
that names sudoku or pawdoku (T00 step 4); the `[default.extend-words]` in `_typos.toml`
keeps `mis` (the tickets use the hyphenated prefix) and drops `als`, the sudoku abbreviation, unless T00 finds a file that needs it.
`.markdownlint-cli2.jsonc` adds one delta: `MD033`'s `allowed_elements` becomes
`["br", "important"]`, because `AGENTS.md` uses `<important if>` blocks (§7). T05 cannot
make this change, because the file freezes at T00. `.gitignore` also ignores Claude
Code's runtime state under `.claude/` (T00 step 4).
`taplo.toml` is T02's; the other six were final at T00. Two moved later: T10 added `ede`
to `_typos.toml`'s words, and `.gitignore` gained `.allium-loop/` (T11), `mutants.out/`
(T15) and the fuzz crate's outputs (T16), which `.dockerignore` mirrors by hand.

`just initialize` never formats. T00 drops L `scripts/initialize.sh`'s `just format`
line and its comment, because a first-run script that rewrites files hides drift that the
gate should report. A carried page or skill that says initialization normalises
formatting drops that clause. On 2026-10-24 the script's toolchain comment lost "and
targets", as `install-toolchain`'s did (§4).

## 6. Handbook

The Diátaxis handbook under `docs/`, under the documentation contract L ships
(L `docs/reference/documentation-contract.md`). `docs/manifest.yml` and `docs/README.md`
were final at T00 and frozen in wave 1; wave 2 registered its own page and records in
both. T00 writes the manifest as L
`docs/manifest.yml` minus the entries for the dropped pages and records, with the
decision records renumbered and retitled as §8 lists, in this order: L's grouping, then
decisions last. Each registered page has a stub at T00 with valid frontmatter equal to its
manifest entry, a matching H1 and at least 40 words of real prose naming what the page
will say and which lane writes it. Carried pages keep L's `canonical_for` slugs.

| Path | Owner | From L | Adaptation |
| --- | --- | --- | --- |
| `docs/README.md` | T00 | same | A map for a two-crate workspace with one placeholder module; no domain section |
| `docs/project/purpose-and-scope.md` | T07 | same | What the foundation is, what it is not (no domain, no runtime, no publishing), who consumes the library |
| `docs/project/repository-map.md` | T07 | same | The tree of §3 |
| `docs/project/terminology.md` | T07 | same | The glossary seed: repository, library, service, effect, effect trait, fake, specification, gate; no domain words |
| `docs/tutorials/first-change.md` | T07 | same | A domain-neutral exercise that can be followed again: change the line `run()` writes, watch its unit test and the binary test fail, update both, `just check` |
| `docs/how-to/develop-locally.md` | T07 | same | Two crates; otherwise L's |
| `docs/how-to/test-and-debug.md` | T07 | same | nextest filters per crate, the binary test, coverage HTML |
| `docs/how-to/maintain-dependencies.md` | T07 | same | L's; "nothing updates them" until T17, which replaced it: Renovate now proposes each update and holds it for dashboard approval (decision 0018) |
| `docs/how-to/deploy-to-github-pages.md` | T07 | same | Both crates' rustdoc, `BiscuitBin/pachinko`'s Pages URL |
| `docs/how-to/work-with-the-specs.md` | T06 | same | The module table has the one placeholder; the waiver terms verbatim |
| `docs/explanation/specifications.md` | T06 | same | Specifications are the source of truth; there is one module and it states no behaviour yet |
| `docs/explanation/architecture.md` | T07 | same | The lib/service split, the effects boundary and who implements it, what is deferred (decision 0014), and what wave 2 added (T13, T14); sentences the wave-2 tickets left in the future tense were brought up to date on 2026-10-24 |
| `docs/explanation/layering.md` | T07 | same | The crate direction Cargo enforces; module direction follows the spec import graph when there is one |
| `docs/explanation/quality-philosophy.md` | T07 | same | L's argument; examples from this gate |
| `docs/explanation/security-model.md` | T07 | same | A library plus a service binary: supply chain, `forbid(unsafe_code)`, cargo-deny, the effects boundary; the service's network surface is deferred |
| `docs/reference/commands.md` | T07 | same | The recipe table of §4 |
| `docs/reference/configuration.md` | T07 | same | Both manifests, the lint table, `deny.toml`, the floor, the pixi manifest, both tool lists, the effects script |
| `docs/reference/testing.md` | T07 | same | nextest, doctests, the binary test, coverage scope, fakes through the trait, the insta rule waiting for its first snapshot |
| `docs/reference/quality-gates.md` | T07 | same | The nineteen gates of §4 |
| `docs/reference/documentation-contract.md` | T07 | same | L's rules verbatim; "Links that leave the repository" names only the outward pages that exist here: decision 0005, `api.md`, `deploy-to-github-pages.md` |
| `docs/reference/agent-contract.md` | T07 | same | Fourteen skills |
| `docs/reference/api.md` | T07 | same | rustdoc for both crates, `just doc`, `just site`, Pages |
| `docs/operations/maintenance.md` | T07 | same | L's routine |
| `docs/operations/troubleshooting.md` | T07 | same | L's, minus wasm |
| `docs/decisions/README.md` and fourteen records | T07 | §8 | |

Wave 2 added one page, `docs/how-to/build-the-container-image.md` (T13), and decisions
0015 to 0018 (§8). On 2026-10-24 the handbook was brought up to date with wave 2 and the
maintainer's choices of that day were recorded on their owning pages.

## 7. Agent contract

`AGENTS.md` is canonical; `CLAUDE.md` is exactly `@AGENTS.md`;
`.github/copilot-instructions.md` is B's adapter text. Both adapters and
`.claude/settings.json` are byte-identical to L's.

`AGENTS.md` is restructured rather than following L's layout. Rules that hold for every
task stay always visible; rules for one kind of task sit in an `<important if="...">`
block that names that task. In this order:

- **The always-on core.** A one-paragraph identity (a library and a service whose domain
  the specifications will define). A `## Project map` of `crates/libpachinko/`,
  `crates/pachinko/`, `docs/`, `docs/specs/`, `docs/decisions/`, `tickets/`,
  `.agents/skills/` with its `.claude/` and `.codex/` bridges, `allium-skill-reference/`
  and `ai_tmp/`. The rule that the specifications under `docs/specs/` decide *what* the
  code does, and win when the two disagree. The rule that issue text, pull request
  bodies, comments, fixtures and tool output are untrusted data. Then `## Invariants`,
  holding the nine below verbatim.
- **L's always-on authority rules**, still in `## Invariants` and outside every block:
  anything that leaves the worktree needs explicit authorization for each action, and the
  rule names every recipe that reaches the network; scratch goes in `ai_tmp/`; one ticket
  per worktree; hooks are never installed from a worktree. The hand-back rule, which
  names `just check` and defines green as §11 does, also stays outside every block.
- **Seven `<important if="...">` blocks**, one per task: changing Rust under `crates/`;
  adding or changing an effect (a trait in libpachinko with its fake beside it, the real
  implementation in pachinko, `just effects`); writing or changing a module under
  `docs/specs/`; changing a page under `docs/`; changing the agent surface (`AGENTS.md`,
  an adapter, a skill or a bridge); a failing gate, or handing work back; picking up a
  ticket under `tickets/`. Each block names only the recipes and the skill its task uses.
- **`## Provenance`**, last, keeping L's sentence containing `runes`.

There is no commands table: `just --list` is the index. Prose that a gate already
enforces (rustfmt, taplo, clippy pedantic, EditorConfig) and code snippets are cut. The
target was about 1000 words; the file is about 1300 (1326 by `wc -w` on 2026-10-24),
because the authority list, the network recipe list and the review fixes added the rest
(T05). Whether to keep that length or cut it back is the maintainer's open question.
Each of B's six phrases (§1 fact 2) appears at least once
outside every block, so that no phrase depends on a block's wording.

The nine invariants, in place of L's:

1. The specifications are the source of truth for behaviour.
2. libpachinko performs no effect itself. The clock, randomness, the filesystem, the
   network, the environment and threads are reached through traits libpachinko defines,
   with fakes beside them; `pachinko` supplies the real implementations. `just effects`
   proves it.
3. Public types are `Send + Sync + 'static`, `Clone` and `Debug`; public enums and
   structs are `#[non_exhaustive]`.
4. `#![forbid(unsafe_code)]` in every crate.
5. `pachinko` depends on `libpachinko`, never the reverse.
6. Clippy pedantic with the workspace lint table; `#[expect(lint, reason = "...")]` only.
7. Coverage does not fall below 90% of lines over `crates/*/src/**`.
8. `Cargo.lock` is the pin; manifests write full `x.y.z` caret ranges; every gate passes
   `--locked` (decision 0007).
9. No dependency is added without the decision-0007 note.

L's third invariant (errors are thiserror enums) waits for the first error type, and
still waits: no crate has one. The change that adds it adds thiserror with its
decision-0007 note and the invariant to `AGENTS.md`, since `just deps-unused` rejects
thiserror before anything uses it. L's fifth (wasm) is dropped. The Provenance section
keeps L's sentence containing `runes`.

Skills, canonical under `.agents/skills/<name>/SKILL.md`, each with a bridge under
`.claude/skills/` and `.codex/skills/` whose body is B's `BRIDGE_BODY`. The bridges
repeat the canonical frontmatter, which B's `_check_skills` compares, and they freeze at
T00, so T00 writes the final frontmatter: L's for every skill except `rust-change`, whose
description, in the canonical file and both bridges, is `Implement or review a change to
either crate with the effects boundary, tests and documentation evidence the invariants
require.`

| Skill | From L | Change |
| --- | --- | --- |
| `rust-change` | same | Two crates; the effect trait goes in libpachinko and the real implementation in pachinko |
| `spec-change` | same | One placeholder module today |
| `plan-change`, `review-change`, `review-docs`, `project-check` | same | Gate names and numbers of §4; no wasm |
| `fix-quality` | same | A dispatch line for every gate of §4, not only L's; no wasm |
| `allium`, `distill`, `elicit`, `propagate`, `tend`, `weed`, `witness` | same | Byte-for-byte from L except recipe names |

`skills-lock.json` is not carried. Its `computedHash` is the skills CLI's hash of the
whole upstream skill folder, so it cannot be checked against the vendored files: for
`allium`, the lock records `52446c61…` and L's `SKILL.md` hashes to `2445a3b1…` under
sha256. A local recomputation would put a different meaning into a field the CLI reads.
The lock names no tag or commit, and no gate reads it. T05 does not create it, and no
page or decision describes it as a pin or a checksum.

Fourteen skills, twenty-eight bridges. `just check-agents` prints
`Validated AGENTS.md, 2 adapters, and 14 skills.`

## 8. Decision records

A fresh series. A carried record opens "Carried from libpawdoku's decision NNNN at
`ccdd001`" and keeps L's `canonical_for` slug. Format as L's: frontmatter,
`# Decision NNNN: title`, Context, Decision, Consequences, What would reopen this,
Related pages. T07 wrote 0001 to 0014; each wave-2 ticket wrote its own.

| Number | File | Title | From L | Slug |
| --- | --- | --- | --- | --- |
| 0001 | `0001-library-and-service.md` | A library and a service in one workspace | new | `decision_library_and_service` |
| 0002 | `0002-specs-are-the-source-of-truth.md` | Specifications decide behaviour | 0002 | `decision_spec_first` |
| 0003 | `0003-effects-behind-traits.md` | Effects behind traits | 0003, adapted: every effect, not randomness alone; the service implements them | `decision_ports_and_fakes` |
| 0004 | `0004-hook-runner-and-checkers.md` | Hook runner and checkers | 0004 | `decision_python_toolchain` |
| 0005 | `0005-project-managed-allium-cli.md` | A project-managed Allium binary | 0005 | `decision_allium_cli` |
| 0006 | `0006-apache-2-0.md` | Apache-2.0 | 0006 | `decision_license` |
| 0007 | `0007-dependency-policy.md` | Dependency policy: caret ranges and `Cargo.lock` | 0007 | `decision_dependency_policy` |
| 0008 | `0008-std-library.md` | A std library, with effects kept out mechanically | new, in place of L's 0008 | `decision_std_library` |
| 0009 | `0009-rust-quality-gate.md` | The Rust quality gate | 0009 | `decision_rust_gate` |
| 0010 | `0010-skill-reference-material.md` | Skill reference material beside the skills | 0010, corrected: no `skills-lock.json` (§7) | `decision_skill_reference` |
| 0011 | `0011-tool-manager.md` | Tool manager | 0011 | `decision_tool_manager` |
| 0012 | `0012-metrics-gate.md` | The metrics gate | 0013 | `decision_metrics_gate` |
| 0013 | `0013-api-reference-on-github-pages.md` | The API reference on GitHub Pages | 0016 | `decision_api_reference_on_pages` |
| 0014 | `0014-service-runtime-deferred.md` | The service runtime waits for the specifications | new | `decision_service_runtime_deferred` |
| 0015 | `0015-container-image.md` | The container image | T13 | `decision_container_image` |
| 0016 | `0016-release-process.md` | The release process | T14 | `decision_release_process` |
| 0017 | `0017-mutation-bench-and-fuzz.md` | Mutation testing, benchmarks and fuzzing | T15 and T16 | `decision_mutation_bench_fuzz` |
| 0018 | `0018-renovate.md` | Dependency updates through Renovate | T17 | `decision_dependency_updates` |

0003 and 0008 divide the effects argument: 0003 is the pattern (traits in the library,
fakes beside them, real implementations in the service); 0008 is why std and not no_std,
and what keeps the effects out instead (gate 8, §9). 0012's thresholds are L's 0013's;
its module-boundary rules wait for modules. 0017 is shared by two wave-2 tickets:
whichever lands first creates the record, its manifest row and its index line with its
own section; the other rebases, or merges `main`, and adds its section. T15 landed
first; T16 merged `main` (`a86d69d`) and kept both sections.

## 9. The placeholder module and the effects boundary

### `docs/specs/pachinko.allium` (T00 writes it; T06 finalises it)

The root module, `-- allium: 3`, in the header shape of L `docs/specs/sudoku.allium`
lines 1-47 (title, Scope, Includes, Excludes, Dependencies, Vocabulary as comments).
Scope says the domain is not yet defined and this module is where it will be; Includes
says nothing; Excludes says every behaviour, until a specification change defines one;
Dependencies says none. It declares no entity, rule, surface or invariant. `just
check-specs` and `just analyse-specs` must each print one block with empty `diagnostics`
and `findings` (§12 claim 1). If allium refuses a module with no declarations, the
fallback is the smallest declaration the language reference allows that states no
behaviour, recorded in T00's hand-back notes and in the module's own comment. Outcome
(T00, T06): allium accepted the module with no declaration, so no fallback was needed.

### The effects boundary (T10)

What is banned, in libpachinko only:

- **Crates**, through its normal dependency tree: `rand`, `rand_core`, `getrandom`,
  `tokio`, `async-std`, `smol`, `mio` and `socket2`, and any crate T10 adds with a
  reason. T10 added none, and the maintainer chose on 2026-10-24 to keep these eight as
  a denylist rather than an allowlist of libpachinko's tree. Development dependencies are
  exempt, so a test may use a seeded generator.
- **std APIs**: `std::time::SystemTime::now`, `std::time::Instant::now`,
  `std::time::Instant::elapsed`, `std::time::SystemTime::elapsed`, everything under
  `std::fs`, `std::env`, `std::net` and `std::process`, `std::thread::spawn`,
  `std::thread::scope`, `std::thread::sleep` and `std::thread::Builder`. Standard I/O
  (`std::io::stdin` and the rest) is not banned: decision 3 does not name it.

The mechanism was T10's choice among these, decided by evidence (§12 claim 2); the
outcome below records what it chose:

- **Crates:** a `cargo tree -p libpachinko -e normal --all-features --target all --prefix
  none --locked --offline` line in `scripts/check-effects.sh` that must match none of the banned names (covers
  transitives, scoped to libpachinko by construction). It is the only crate guard;
  `deny.toml` bans no effect crate (§1 fact 3).
- **std APIs:** either clippy's `disallowed-methods` and `disallowed-types` in a
  `crates/libpachinko/clippy.toml`, if clippy reads that file for libpachinko alone
  (it then restates every key of the root `clippy.toml`, because clippy uses the first
  file it finds and merges nothing, if that is what T10 finds), or rustqual
  `[[architecture.pattern]]` rules with `forbid_path_prefix` in a second configuration,
  `rustqual-effects.toml`, run over `crates/libpachinko` only from the script. They
  cannot go in `rustqual.toml`, because `just metrics` runs that file over
  `crates/pachinko` too, where every effect is allowed.

Whatever the mechanism, `scripts/check-effects.sh`:

1. Checks libpachinko and exits non-zero on any finding.
2. Checks `tests/fixtures/effects-violation/`, which breaks every ban exactly once, in the
   form the tool resolves (a `use std::fs; fs::read(..)` as well as a full path where a
   prefix rule would miss one, and a method call such as `i.elapsed()`, which has no path
   for a prefix rule to match), and fails unless every named ban fires there. `elapsed`
   is broken as `fn f(i: Instant) { i.elapsed(); }`, so that the `now` ban does not fire a
   second time. The crate
   bans are probed by the same `cargo tree` line, run over the fixture with
   `--manifest-path`: the fixture is its own workspace with a committed `Cargo.lock`, and
   each banned crate is an offline path-dependency stub, one of them reached only
   transitively (T10 step 4).
3. Checks that no ban fires on `crates/pachinko`, which is allowed every effect.

T00's version of the script did step 1 for crates only (the `cargo tree` line over an
empty dependency tree) and printed that the std-API bans and the probe arrived with T10,
so gate 8 was green and honest from the first commit. T00's review widened the crate
line with `--all-features --target all`, so optional dependencies and those of another
platform are seen too.

**Outcome (T10, carried by T11).** T10 chose clippy for the std APIs. Clippy reads
`crates/libpachinko/clippy.toml` for libpachinko alone and shadows the root file, so that
file restates every root line, and the script's check 0 fails when the two drift.
rustqual's `forbid_path_prefix` was rejected: it matches text, so it missed `use std as
s`, a glob, a macro body and a renamed import, and could ban `elapsed` only by name.
Clippy takes item paths, not modules, so "everything under" a module is a list of its
stable items: 121 entries with reasons beginning `effect:`, wider than the list above
where a path reaches the same effect (the `Path` filesystem queries, the unix `fs` and
`net` extensions, `RandomState`, `available_parallelism`, the timed `std::sync` waits). The
script runs four checks: 0, the drift check; then the three above, with clippy run three
times over libpachinko (every feature, none, and every feature in the release profile).
Known limits, each recorded in T10's Open points: a constant cannot be banned, so
`std::env::consts` stays reachable; code behind another target's `cfg`, a mix of
features, `cfg(not(clippy))` or a custom `cfg`, and std items newer than the list are
left to review; `Backtrace::capture` stays allowed by the maintainer's choice. On
2026-10-24 the maintainer accepted `std::env::consts` staying reachable and another
target's `cfg` staying review's, and kept the bans over libpachinko's own tests, so a
test that needs a real effect belongs under `crates/pachinko/tests/`; decision 0008
records each. T18 item 12 compared the list with the 1.99.1 documentation of `std::fs`,
`std::env`, `std::net` and `std::process`; a toolchain bump re-runs that inventory
(§13).

## 10. CI

L `.github/workflows/ci.yml`, `audit.yml`, `pages.yml` and `.github/actions/setup/action.yml`
at `ccdd001`, with these changes:

- The `wasm` job is deleted, and leaves `check`'s `needs`. The `rust` job, which already
  runs `metrics` and `snapshots-check`, gains `just effects` after `just metrics`.
- `pages.yml` publishes `just site`, which now links both crates.
- The setup action installs every tool except allium; L's description says "The allium
  binary is the documents job's own step". Any job that runs `just check`, `lint`,
  `check-specs` or `analyse-specs` runs `just install-allium` after the setup action.
  T00's one-job stub did until T01 replaced it; today those jobs are L's `documents` job
  in `ci.yml` and the `verify` job of T14's `release.yml`.
- Every remote `uses:` keeps a full SHA with a version comment, re-read on the day T01
  runs (`gh api repos/<owner>/<repo>/git/ref/tags/<tag>`, dereferencing annotated tags).
  The local `uses: ./.github/actions/setup` has no pin.
- Every gate job that runs recipes, `rust`, `coverage`, `deny` and `documents`, ends with
  `just check-clean`, so a gate step that rewrites a tracked file fails CI. L's jobs did
  not; the maintainer added the step on 2026-10-24. `concurrency` still cancels an
  in-progress run on `main`, as L's does, by the maintainer's choice of the same day.

The required check is `check`, the aggregate job, from T01 onward and never anything
else. A new gate job joins `check`'s `needs` and nothing else moves. In wave 2, T13's
`image` job joined `needs`, because a Dockerfile that no longer builds is a broken
service; T14's `semver` job (a report on pull requests, as L's draft argues), the
scheduled `mutants.yml` and `fuzz.yml`, and the tag-triggered `release.yml` never do.
`audit` is never in `needs`.

## 11. Rules for tickets and lanes

- **Worktrees and branches.** Each ticket runs on the branch its `branch:` field names
  (`ticket/<id>-<slug>`, lowercase) in its own worktree, created from `main` after every
  ticket it depends on has merged. From a Supacode terminal:
  `supacode repo worktree-new --branch <branch> --base main --name <id>`; otherwise
  `git worktree add ../<id> -b <branch> main`. The tools live in the worktree's own
  gitignored `.pixi/` and `.tools/`, so every new worktree runs `just initialize` (network,
  authorised); only the primary checkout's run installs the shared hook shim, once
  (`README.md`, "How to pick up a ticket"). The bootstrap's primary checkout ran it for
  T18 item 1, at `7dbed57`.
- **T00 ships a shape-complete skeleton.** `bg-validate-docs` and `bg-validate-agents`
  can only be green if every page, record and skill exists from the first commit, so T00
  ships a stub for each. Lanes **replace** stubs; they never add, rename or reclassify a
  registered page, record or skill. A lane may create only the paths §3 marks `new` for it.
- **Files no wave-1 lane touches:** `Justfile`, `.pre-commit-config.yaml`,
  `.pre-commit-fix.yaml`, `rust-toolchain.toml`, `tools.txt`, `tools-source.txt`,
  `pixi.lock`, `pyproject.toml`, `.editorconfig`, `.gitattributes`, `.gitignore`,
  `.markdownlint-cli2.jsonc`, `lychee.toml`, `_typos.toml`, `scripts/initialize.sh`,
  `docs/manifest.yml`, `docs/README.md`, `CLAUDE.md`, `.github/copilot-instructions.md`,
  `.claude/settings.json`, `crates/pachinko/src/main.rs` and every bridge. A lane that
  needs a change there stops and hands it back as a T00 follow-up pull request on `main`.
  One lane did not: T10 added `ede` to `_typos.toml` on its branch (§5).
- **No dependency is added in wave 1.** None was. Wave 2's harnesses then added divan and
  libfuzzer-sys, each with its decision-0007 note (§1 fact 6).
- **No path appears in two wave-1 lanes' Files-touched lists.** An agent that finds a
  need to edit another lane's file hands it back instead.
- **Wave 2 shares files by rebasing or merging.** The frozen list lifted after T11. A
  wave-2 ticket may then edit its own lines in any file, frozen or owned, and lists each
  such file in its Files touched as `shared, own lines`. The ones most of them touch are
  `Justfile`, `pyproject.toml`, `pixi.lock`, `tools.txt`, `.gitignore`,
  `docs/manifest.yml`, `docs/README.md` (a new page or record must be registered and
  reachable),
  `docs/decisions/README.md`, `docs/reference/commands.md` and `CHANGELOG.md`. Wave-2
  tickets merge one at a time, each rebased onto, or merged with, the last: T16 merged
  `main` twice (`a86d69d`, `853b259`). `pixi.lock` and `Cargo.lock` are regenerated after
  the rebase or merge, never merged by hand, and the ticket's verification is rerun and
  requoted at the resulting commit. A recipe a wave-2 ticket adds sits outside `check`'s
  recipe list; pre-landing wave-2 recipes in T00 does not work, because the fuzz crate's
  `--manifest-path` lines in `sync`, `lock`, `lock-check`, `fmt-check` and `clippy` would
  fail before `fuzz/` exists. Each wave-2 ticket's first acceptance criterion is its
  recipe's green output, quoted.
- **Self-contained.** A ticket is written for an agent with no context: it embeds exact
  content or cites this document by section, and names the L, B or T paths to read. It
  never cites a chat transcript, a scratch directory or a tool result.
- **Paths as code spans.** Ticket text writes repository paths and URLs as code spans,
  never as Markdown links: the hook gate runs lychee offline over `tickets/`.
- **Definition of done**, for every build ticket: `just check` green in this repository,
  on the ticket branch rebased onto, or merged with, the current `main`; the ticket's
  acceptance criteria met; its verification commands run with their output quoted in the
  hand-back notes; the adversarial reviews the ticket names run (§14) with every valid
  finding fixed or handed back; and the ticket's `status:` set to `done` in the same pull
  request. A ticket is done when its gate passes, not when its files exist.
- **Green** means two things together: `just check`'s output ends with the runner's line
  `All checks passed and the worktree is unchanged.` (B
  `src/biscuit_games_tooling/run_project_check.py` line 109), and `git status --short`
  prints nothing after the ticket's commit. Except in T00, `just check` also runs on the
  untouched worktree before any edit, and its last line is quoted as the baseline; a red
  baseline means `main` is broken, and the fix is a `main` pull request, not this lane's
  work. Before asking to push, the agent runs `git fetch origin` (network, authorised),
  rebases onto, or merges, `origin/main`, reruns `just check`, and reruns and requotes
  the ticket's verification. Two lanes that are each green can merge into a red `main`,
  and only this rerun sees them together. The maintainer chose a merge of `origin/main`
  for T10 (`6f54c20`), and T16 merged `main` too (`a86d69d`, `853b259`); T11's fresh
  clone of `46c2e33`, which held every wave-1 lane, was green.
- **Quoted evidence names its commit.** Each quoted output is headed by the commit it ran
  at (`git rev-parse --short HEAD`, plus "with uncommitted changes" when that is true). A
  rebase, a merge of `main` or a §14 fix round after a quote makes it stale: rerun it and
  replace it. CI evidence names the run URL and the commit the run tested, which for a
  pull request is the merge commit, not the branch head.
- **Checks fail closed.** A failed read, a timeout or an empty pipe is never a pass.
  Quoted output shows each command's exit status. A check that passes by printing
  nothing is preceded by a positive control: the unfiltered command, run alone, exits 0
  and matches something. An API read counts as "absent" only on an HTTP 404 whose
  message names absence; GitHub also answers 404 to a token that may not read the
  resource, so that, and any other error, stops the step.
- **Separately authorised actions.** Commits on the ticket branch are the ticket's work.
  Pushing, opening a pull request, tagging, filing issues, changing repository or
  organisation settings, installing a GitHub App, publishing anywhere, installing
  anything outside the worktree, and editing another repository are each separately
  authorised: the ticket says where one occurs, and the agent stops and asks.
- **Network.** `pixi install`, `just install-tools` (and a rustqual source build),
  `just install-allium`, `just install-hooks`, `cargo fetch` and `rustup toolchain install`
  need the network. Each is a named, authorised step in the ticket that runs it.
  `just check` must not need the network. A claim that something works with the network
  blocked is shown under an OS-level denial: on macOS,
  `sandbox-exec -p '(version 1)(allow default)(deny network*)' <command>`; on Linux,
  `unshare -rn <command>`, in a namespace holding only `lo`. Inside the same denial, the
  negative control `curl -fsS --max-time 10 https://static.rust-lang.org` and
  `curl --noproxy '*' -fsS --max-time 10 https://1.1.1.1` must both fail, and the
  hand-back quotes both exit codes. Proxy variables are not denial: DNS, git, rustup and
  raw sockets ignore them. A sandbox that fails to launch is not evidence. If the agent's
  own sandbox blocks a gate recipe, request the permission and record it; never weaken
  the recipe.
- **Scratch.** Anything that is not a deliverable goes in `ai_tmp/` (gitignored from T00)
  or the session's scratch directory, never in a commit.
- **Credentials.** No file carries a token. CI uses the run's own `github.token`.

## 12. Unverified claims, and the ticket that checks each

Each claim was reasoned from source or documentation but not executed here. The named
ticket runs the check and records the outcome in its hand-back notes; a claim that fails
is a design change that comes back through this document. T11 carried the outcomes of
the claims wave 0 and wave 1 own (1 to 6, 10 to 12) into this table; T17, the only
ticket after T13, T14 and T16, carried claims 7 to 9 from their hand-back notes; and
claim 12's Linux half was carried from T18 item 7 on 2026-10-24. L's own
§12 claims about pixi, prek, binstall and the hooks were settled in L; a ticket re-checks
them only by running the gate.

| # | Claim | Check | Owner | Outcome |
| --- | --- | --- | --- | --- |
| 1 | allium 3.6.1 reports empty `diagnostics` and `findings` for a module with no declarations | `just check-specs`, `just analyse-specs` on §9's module | T00 first, T06 | Holds; no fallback declaration. T00 hand-back (claims table), T06 hand-back; both recipes green in T11's fresh clone, network denied. |
| 2 | Clippy reads `crates/libpachinko/clippy.toml` for libpachinko alone and shadows rather than merges the root file; rustqual pattern rules can be scoped to one crate by running it over that directory | A deliberate violation in a scratch copy, each mechanism in turn | T10 | The clippy half holds: the crate file shadows the root one (every root key goes silent for libpachinko), and the same call in pachinko fires nothing. T10 hand-back, Step 2. The rustqual half was not settled: rustqual ran over a scratch crate only, missed four of eleven forms, and was rejected, so the claim is moot. |
| 3 | L's lint table is green on the new stable | `just clippy` on T00's pin | T00, T02 | Holds on 1.99.1 after one assertion change (T00), with every `clippy.toml` key valid and a mistyped key failing (T02). L's own tree was not linted. |
| 4 | An integration test running `CARGO_BIN_EXE_pachinko` makes `main.rs` count as covered under `cargo llvm-cov nextest`, so the one-line-main shape reaches 90% | `just coverage`, then `cargo llvm-cov report --show-missing-lines` | T00 first, T02 | Holds; `main.rs` 100%, no `--ignore-filename-regex`. T00 and T02 hand-backs. |
| 5 | B's pixi git dependency installs in `BiscuitBin` CI with no credential | T00's stub run on `main`, then T01's full run | T01 | Holds. The fetch with no credential is the stub run `39874411413` on `8b01c0c`, a pixi cache miss; T01's four-job run restored the environment from cache. T01 hand-back, Claim 5. |
| 6 | T `scripts/bootstrap_repo.sh` works on an organisation-owned repository; Pages works on the free organisation plan for a public repository | The script without `--apply`, then with it | T01 | Holds with repository admin rights; no organisation-owner action was needed. T01 hand-back, State after; protection read back unchanged by T11. |
| 7 | cargo-semver-checks has a usable baseline before the first release (`--baseline-rev`) | A scratch run against `main` | T14 | Holds: a Git-revision baseline (the merge base with `main`) works before the first release, discovery selects the last `libpachinko-v*` tag afterwards, and an explicit revision is accepted. T14 hand-back, Semver and claim 7; carried by T17. |
| 8 | cargo-fuzz still works on the pinned stable with `--sanitizer none` | `just fuzz` on the smoke target | T16 | Holds on the 1.99.1 pin with `--sanitizer none`: `just fuzz smoke 20` exited 0 under `sandbox-exec` with network denied, both negative controls failing inside the same denial. T16 hand-back, Claim 8; carried by T17. |
| 9 | docker is available locally and on `ubuntu-latest` runners | `docker version`; the first CI run | T13 | Holds: locally, Docker 29.4.0 client and server on OrbStack; on `ubuntu-latest`, the `image` job of run `40065797710` built and ran the image. T13 hand-back; carried by T17. |
| 10 | `just snapshots-check` is green on a workspace with no `insta` dependency and no `.snap` file | The recipe on T00's tree | T00 first, T02 | Holds, and the gate fires on an unreferenced `.snap` (T02's scratch commit). T00 and T02 hand-backs. |
| 11 | rustqual 1.8.3 runs over a crate with no `[[architecture.pattern]]` table and two tiny files without a finding | `just metrics` | T00 first, T10 | Holds on both crates. T00 and T10 hand-backs. |
| 12 | `cargo tree -p libpachinko -e normal --all-features --target all --locked --offline` runs offline after `just sync`, so gate 8 does not need the network | `just effects` under §11's network denial, with its negative control | T00 first, T10 | Holds: `just effects` with a cold `target/effects` and both controls failing (T10, at `2ebacac`), and the whole `just check` in a fresh clone of `46c2e33` under the denial (T11), both on macOS. On Linux, T18 item 7 at `7dbed57`: `just check` exit 0 under `unshare -rn`, the namespace holding only `lo` and both §11 curl controls failing (exit 6 and 7), in an emulated `linux/amd64` Ubuntu 24.04 container under Rosetta (OrbStack), run with seccomp and AppArmor unconfined so the namespace could launch. It reused the warm `target/` T18 item 3 had just built, so it is not a cold build under denial, and it is not a native Linux runner. |

## 13. Risks every ticket states where it applies

- **The toolchain pin is stale by the time a ticket runs.** A ticket that bumps it bumps
  `rust-version` with it and re-runs `just clippy`, because the lint set moves. It also
  re-runs the std inventory against the new release and bans any new item that reaches
  an effect, with a fixture line that breaks it (§9), because a std item is not banned
  until it is listed. Renovate's `rust toolchain` pull request now carries the bump, and
  `docs/operations/maintenance.md` ("Per toolchain release") holds the procedure.
- **pixi's rust shadows rustup** in a shell that does not source `~/.cargo/env`;
  `check-toolchain` is the alarm (L `tickets/CONVENTIONS.md` §13).
- **The small denominator.** Two crates with three items: one untested branch breaches
  90%. Keep each item trivially and fully tested until the specifications bring code.
- **`--locked` and the snapshot.** Any recipe that rewrites `Cargo.lock`,
  `fuzz/Cargo.lock` or `pixi.lock` fails `check-clean`; `target/`, `.tools/` and `.pixi/`
  are ignored from T00.
- **The network at first run, none in `check`.** As §11.
- **A gate that cannot fire.** A ban with no fixture breaking it proves nothing; gate 8's
  probe is what makes decision 3 mechanical. A lane that weakens the probe hands it back.
- **A red stub run on `main`.** T00's merge is the first time the setup action runs on a
  `BiscuitBin` runner (§12 claim 5). If that run is red and T01 then requires `check`,
  every other lane is blocked until T01's workflows land. T01 stops instead of requiring
  a red check. The run was green (§12 claim 5).
- **Organisation settings.** `BiscuitBin` is an organisation, not a user; branch
  protection, Pages and GitHub App installation may need an organisation owner. T01 and
  T17 say so before acting. T01 needed only repository admin rights (§12 claim 6); the
  Mend app for Renovate was installed on 2026-10-24 for the whole organisation, by the
  maintainer's choice, rather than for this repository alone (decision 0018).
- **Frozen files and follow-ups.** T02, T06 and T10 are the lanes most likely to need a
  recipe changed; each change is a small `main` pull request, and T11 reconciled what
  accumulated.
- **lychee and typos over `tickets/`.** Ticket prose must pass typos and carry no
  relative links.

## 14. Adversarial review

A ticket marks each point where a mistake is expensive to undo with a bold
**Adversarial review** paragraph at the end of a step. There the agent gets a second
opinion from reviewers who did not write the change, and other subagents check each
finding and fix the valid ones before the work goes on. This is a procedure for the
agent running a ticket; it is unrelated to T05's `review-change` skill, which this
repository ships to its own contributors.

- **When.** At each checkpoint a ticket marks: before a file freezes, before each
  "Authorisation required" stop, after a design choice later steps build on, and before
  the ticket's final commit.
- **Who reviews.** Codex with `-m gpt-6-astra`, or Claude Code with
  `--model claude-opus-5-5`. Use the harness the executing agent is not, if it is on
  `PATH`, so that the review does not share the author's blind spots; otherwise use the
  same harness's preferred model. A weaker model is used only if neither is available,
  and the hand-back notes say so. In the agent's own harness, a fresh-context subagent on
  the preferred model is equivalent; this section does not prescribe any harness's
  subagent API. The commands, which the agent confirms with `--help` on the day, run from
  the worktree (`claude` takes no directory flag):

  ```sh
  codex exec -m gpt-6-astra -s read-only -C <worktree> \
    -o ai_tmp/review/<id>-<n>/findings-<lens>.md "<brief>"
  claude -p --model claude-opus-5-5 --permission-mode plan "<brief>" \
    > ai_tmp/review/<id>-<n>/findings-<lens>.md
  ```

  `<id>` is the ticket id, `<n>` the checkpoint's place among the ticket's checkpoints,
  counting from 1, and `<lens>` is `conformance` or `breakage`. A second round (below)
  writes beside the first with a `-2` suffix.
- **Phase 1, review.** Two reviewer subagents run in parallel, each with a fresh context
  and one lens:
  - **(a) Conformance.** The diff against the ticket's Goal, Files touched, acceptance
    criteria and the sections of this document it cites. Deltas from L beyond the stated
    ones. A frozen file or another lane's file touched.
  - **(b) Breakage.** How the gate could pass while the claim is false. A gate that
    cannot fire. What fails on a fresh clone, in CI, offline, or for a later lane. An
    acceptance criterion asserted but not shown. A verification command that would
    print a pass when it failed: a pipeline whose first command errored, so the filters
    print nothing; `|| true` on a line a claim rests on; a wait that succeeds on timeout;
    a 401, 403 or network error read as "absent"; a network denial with no negative
    control (§11).

  The brief gives the ticket's path, this document's path, `git diff main...HEAD` plus
  the uncommitted changes (or the artefact under review, such as a dry run or a list),
  and the checkpoint's questions. It does not give the author's reasoning or the
  hand-back notes' conclusions. Reviewers are read-only. Each finding has an id, a
  `file:line`, the claim, a concrete failure scenario, a severity (high, medium or low)
  and its evidence.
- **Phase 2, triage.** Separate subagents, neither the reviewers nor the executing agent,
  check every finding against the code, by running a command or reading the pinned
  source. Each finding is marked valid, with a fix written, or invalid, with the reason.
  The verdicts go to `ai_tmp/review/<id>-<n>/verdicts.md`. Subagents pass file paths to
  each other, not pasted text.
- **Phase 3, apply.** The triage subagents apply the valid fixes one at a time, never two
  writers at once. Then the executing agent re-runs the checkpoint's verification
  commands, and `just check` at the final checkpoint.
- **Scope guard.** A fix edits only files in the ticket's Files touched. A valid finding
  whose fix lies outside that list, in a frozen file or in another lane's file, is handed
  back under §11 and never applied. At an authorisation checkpoint the fixes change what
  the agent will ask for, and the agent still stops and asks.
- **Rounds.** If any high-severity finding was valid, one more review runs over the fix
  diff. Two rounds at most; anything still open goes into the ticket's Open points.
- **Network.** The review calls a model API. The ticket's checkpoint is the authorisation
  (§11, "Network"). It runs outside `just check`, so `check` still needs no network. A
  brief carries only this repository's content and the L, B and T files the ticket
  cites, never a credential.
- **Record.** The hand-back notes record, for each checkpoint: the harness and model, the
  findings, each verdict with a one-line reason, what was applied, what was handed back,
  and the re-run output, headed by the commit it ran at (§11). The raw files stay in
  `ai_tmp/` and are never committed.
