# Conventions for bootstrapping `pachinko`

This document is the design every ticket under `tickets/` obeys. A ticket cites it by
section (`CONVENTIONS.md §4`) instead of restating it, and embeds exact content only where
the agent executing the ticket would otherwise have to guess. Where a ticket and this
document disagree, this document wins and the ticket is corrected. Changes to this
document go through a pull request on `main`, never through a lane branch, because every
lane reads it.

The foundation is a copy of libpawdoku's, adapted. Almost every file a ticket writes
already exists, finished and green, in L at the pinned commit; a ticket names that file and
the delta, and the agent reads the original. Restating L here would only give the two
copies room to disagree.

## 0. What is being built, and where the sources are

`BiscuitBin/pachinko` (public, on the free organisation plan; today one `first commit`
holding a stub `README.md`, plus this `tickets/` directory on `bootstrap-tickets`) becomes
a Cargo workspace of two crates:

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
work wave 2 does here, and wave-2 tickets adapt those drafts rather than re-deriving them:

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
   tables) takes a module prefix. Which mechanism T10 uses is its decision (§9, §12).
5. **L's live state is the baseline, not L's T00.** L's gate grew after its foundation:
   `metrics` (rustqual, L's T22), `snapshots-check` (insta, L's T29), `plan-spec` (L's
   T24), `site` and `pages.yml` (L's T33). Pachinko's T00 starts from L's files at
   `ccdd001`, so all of these exist from the first commit.
6. **No dependency until the specifications.** cargo-shear (gate `deps-unused`) flags a
   dependency nothing uses, so L's thiserror, serde, proptest and insta are not copied.
   The insta recipes land; the `insta` crate does not until something is snapshotted.
7. **B's pixi git dependency is public.** `steven-cutting/biscuit_games_tooling` stays the
   URL in `pyproject.toml`; only this repository's own URLs move to `BiscuitBin/pachinko`.

## 2. Toolchain and pins

### `rust-toolchain.toml` (T00 creates it; frozen)

```toml
[toolchain]
# An exact stable release, bumped deliberately. `channel = "stable"` would move the
# clippy lint set every six weeks and break `-D warnings` with no diff. rust-version
# in Cargo.toml follows this pin until a crate is first published.
channel = "<the newest stable on the day T00 runs>"
profile = "minimal"
components = ["rustfmt", "clippy", "llvm-tools-preview"]
```

The channel is the newest stable release on the day T00 executes, read from
`https://static.rust-lang.org/dist/channel-rust-stable.toml` (the `[pkg.rust] version`
line). On 2026-10-21 that was `1.99.1`; L pins `1.98.1`. `rust-version` in
`[workspace.package]` is the pin's `major.minor`. Against L: no `targets` line (decision
2), and no `rust-src` (it served rust-analyzer only; a contributor adds it with
`rustup component add`). `llvm-tools-preview` serves cargo-llvm-cov.

Because L's lint table was tuned against 1.98.1, T00 runs `just clippy` on the new pin
before freezing anything, and T02 owns any lint the new stable added or renamed (§12
claim 3).

### `pyproject.toml` (the pixi manifest; T00 writes it, T03 finalises its comments)

L `pyproject.toml` with these changes and no others:

- `[project] name = "pachinko-tooling"`, and the description names pachinko.
- The `recipes` list in `[tool.biscuit-games-tooling]` is §4's gate order.
- Every pin stays as L has it (`just ==1.58.0`, `prek ==0.5.3`, `cargo-binstall ==1.23.0`,
  `cargo-nextest ==0.9.146`, `cargo-llvm-cov ==0.9.1`, `cargo-deny ==0.20.2`,
  `cargo-insta ==1.48.0`, `cargo-shear ==1.13.4`, `taplo ==0.10.0`, Python `3.14.*`, B at
  `v0.3.0`). Moving them is Renovate's job (T17), not the foundation's.

`pixi.lock` is generated by `pixi lock`, committed, and never hand-edited. Pixi owns the
tools and the Python environment; rustup owns the compiler; the environment never gains
`rust` (L `docs/decisions/0011-tool-manager.md`).

### `tools.txt` and `tools-source.txt` (T00 creates them; frozen in wave 1)

L `tools.txt` (`cargo-hack@0.6.45`) and L `tools-source.txt` (`rustqual@1.8.3`), each
with its comment block, verbatim. Wave-2 tickets add their own lines (§11).

### Maintainer prerequisites (not ticket steps)

As L `tickets/CONVENTIONS.md` §2 "Maintainer prerequisites": rustup installed and its
`~/.cargo/bin/cargo` first on `PATH`; pixi 0.81.0 or newer; any just; gh authenticated
with access to `BiscuitBin`. If an agent's shell finds `~/.pixi/bin/cargo` first, every
command is prefixed `PATH="$HOME/.cargo/bin:$PATH"`. docker is a prerequisite for T13
only (§12 claim 9).

## 3. Target repository tree (exact paths and owners)

Owner is the lane that writes the final content. T00 creates every path below that has
T00 in its row, as a stub or in final form, so that `just check` is green on the first
commit. A path that a lane creates is marked `new`; no other lane touches it. No path has
two wave-1 owners. `(frozen)` marks files no wave-1 lane touches after T00 (§11).

```text
Cargo.toml                          T00 skeleton -> T02 (lints, profiles)
Cargo.lock                          T00 (generated; two members, no dependencies)
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
crates/libpachinko/clippy.toml      T10 new, only if T10 chooses clippy (§9)
rustqual-effects.toml               T10 new, only if T10 chooses rustqual (§9)
LICENSE                             T00 (Apache-2.0 text, final)
README.md  CHANGELOG.md  SECURITY.md   T00 stub -> T12
AGENTS.md                           T00 stub (six phrases, 300 words) -> T05
CLAUDE.md  .github/copilot-instructions.md   T00 (byte-identical to L; frozen)
.agents/skills/<14>/SKILL.md        T00 stub -> T05
.claude/skills/<14>/SKILL.md  .codex/skills/<14>/SKILL.md   T00 (fixed bridge body; frozen)
.claude/settings.json               T00 (byte-identical to L; frozen)
skills-lock.json  allium-skill-reference/   T05 new
Justfile                            T00 (frozen in wave 1)
.pre-commit-config.yaml  .pre-commit-fix.yaml   T00 (frozen)
.editorconfig  .gitattributes  .gitignore   T00 working -> T03
.markdownlint-cli2.jsonc  lychee.toml  _typos.toml   T00 working -> T03
pyproject.toml                      T00 -> T03 (comments only; pins and recipes frozen)
pixi.lock                           T00 (generated; frozen in wave 1)
scripts/initialize.sh               T00 working -> T03
.github/actions/setup/action.yml    T00 stub -> T04
.github/workflows/ci.yml            T00 one-job stub -> T04
.github/workflows/audit.yml  .github/workflows/pages.yml   T04 new
docs/manifest.yml  docs/README.md   T00 (frozen in wave 1)
docs/project/*.md                   T00 stub -> T07
docs/tutorials/first-change.md      T00 stub -> T07
docs/how-to/{develop-locally,test-and-debug,maintain-dependencies,deploy-to-github-pages}.md   T00 stub -> T07
docs/how-to/work-with-the-specs.md  T00 stub -> T06
docs/explanation/specifications.md  T00 stub -> T06
docs/explanation/{architecture,layering,quality-philosophy,security-model}.md   T00 stub -> T08
docs/reference/*.md  docs/operations/*.md   T00 stub -> T08
docs/decisions/README.md  docs/decisions/0001..0014-*.md   T00 stub -> T09
docs/specs/pachinko.allium          T00 placeholder -> T06
tickets/                            this directory
ai_tmp/  .tools/  .pixi/  target/   gitignored; never committed
```

Not carried from L: `crates/pawdoku/**`, every `docs/specs/*.allium` (the nine sudoku
modules), `docs/explanation/{human-solving,solving-sudoku,puzzle-design}.md`, L's decisions
0008 (no_std), 0012, 0014 and 0015, `tests/fixtures/metrics-violation/` (its rules were
sudoku layering), and every snapshot.

Wave 2 creates paths of its own; each wave-2 ticket lists them.

### The two crates (T00 writes them; T02 finishes them)

Root `Cargo.toml` is L `Cargo.toml` with: `repository` and `homepage` set to
`https://github.com/BiscuitBin/pachinko`; `rust-version` per §2; no
`[workspace.dependencies]`; the `std_instead_of_core`, `std_instead_of_alloc` and
`alloc_instead_of_core` lines removed (decision 2); the `module_name_repetitions`
comment reworded without the sudoku example; the `[profile.release]` comment about pyo3
removed. T00 lands the full lint table at once, because `-D warnings` is decided by it
and a skeleton with fewer lints would only move the failures into T02.

`crates/libpachinko/Cargo.toml`: L `crates/pawdoku/Cargo.toml`'s package shape, every
field `.workspace = true`, `name = "libpachinko"`, a domain-neutral `description`
(the `cargo` lint group requires one, plus `keywords`, `categories` and `readme`), no
`no-std` category, no `[features]` and no dependencies. `src/lib.rs`: the crate docs
(stating that the domain arrives with the specifications) and one item,
`pub const fn version() -> &'static str` returning `env!("CARGO_PKG_VERSION")`, with a
doctest and an in-module unit test, so that every gate has something to format, lint,
test, document and measure. `env!` is read at compile time and is not the effect
decision 3 bans.

`crates/pachinko/Cargo.toml`: the same shape, `name = "pachinko"`, one dependency,
`libpachinko = { path = "../libpachinko", version = "0.1.0" }`. Cargo then enforces the
direction: `libpachinko` cannot name `pachinko` without a cycle. The package has a
library target and a binary target:

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

L `Justfile` at `ccdd001`, with exactly these changes:

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
  # pachinko crate. The bans and the mechanism are the script's; this recipe
  # only runs it.
  effects:
      sh scripts/check-effects.sh
  ```

- `site` writes a root `index.html` that links to both crates' pages
  (`libpachinko/index.html` and `pachinko/index.html`) instead of redirecting to one, with
  the title "pachinko: API reference".
- No other recipe changes. `snapshots-check`, `snapshots-review`, `snapshots-accept`,
  `plan-spec`, `install-tools`'s two passes, `audit`, `check-links-online` and
  `check-clean` are L's text.

`coverage` is not changed to ignore `main.rs`. If T00 finds that the binary's lines are
not counted (§12 claim 4), T00 adds `--ignore-filename-regex 'crates/pachinko/src/main\.rs'`
to the `coverage` recipe's `report` lines before the Justfile freezes, and records why.

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
`check`, as in L. Wave 2 adds recipes outside `check` only (§11).

## 5. Hooks and dotfiles

`.pre-commit-config.yaml` and `.pre-commit-fix.yaml` are L's at `ccdd001`, byte for byte
(neither names a crate; their `files:` patterns already read `crates/.*`). Every remote
hook keeps L's rev and comment. Both are frozen at T00.

`.editorconfig`, `.gitattributes`, `.gitignore`, `.markdownlint-cli2.jsonc`,
`lychee.toml`, `_typos.toml` and `taplo.toml` are L's at `ccdd001`. T00 copies them as they
are, then removes only what names sudoku or pawdoku; the `[default.extend-words]` in
`_typos.toml` keeps `mis` (the tickets use the hyphenated prefix) and drop `als`, the
sudoku abbreviation, unless T03 finds a file that needs it. `taplo.toml` is T02's; the other six are T03's.

## 6. Handbook

The Diátaxis handbook under `docs/`, under the documentation contract L ships
(L `docs/reference/documentation-contract.md`). `docs/manifest.yml` and `docs/README.md`
are final at T00 and frozen in wave 1. T00 writes the manifest as L
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
| `docs/how-to/maintain-dependencies.md` | T07 | same | L's; "nothing updates them" until T17 |
| `docs/how-to/deploy-to-github-pages.md` | T07 | same | Both crates' rustdoc, `BiscuitBin/pachinko`'s Pages URL |
| `docs/how-to/work-with-the-specs.md` | T06 | same | The module table has the one placeholder; the waiver terms verbatim |
| `docs/explanation/specifications.md` | T06 | same | Specifications are the source of truth; there is one module and it states no behaviour yet |
| `docs/explanation/architecture.md` | T08 | same | The lib/service split, the effects boundary and who implements it, what is deferred (decision 0014) |
| `docs/explanation/layering.md` | T08 | same | The crate direction Cargo enforces; module direction follows the spec import graph when there is one |
| `docs/explanation/quality-philosophy.md` | T08 | same | L's argument; examples from this gate |
| `docs/explanation/security-model.md` | T08 | same | A library plus a service binary: supply chain, `forbid(unsafe_code)`, cargo-deny, the effects boundary; the service's network surface is deferred |
| `docs/reference/commands.md` | T08 | same | The recipe table of §4 |
| `docs/reference/configuration.md` | T08 | same | Both manifests, the lint table, `deny.toml`, the floor, the pixi manifest, both tool lists, the effects script |
| `docs/reference/testing.md` | T08 | same | nextest, doctests, the binary test, coverage scope, fakes through the trait, the insta rule waiting for its first snapshot |
| `docs/reference/quality-gates.md` | T08 | same | The nineteen gates of §4 |
| `docs/reference/documentation-contract.md` | T08 | same | L's rules verbatim; "Links that leave the repository" names only the outward pages that exist here: decision 0005, `api.md`, `deploy-to-github-pages.md` |
| `docs/reference/agent-contract.md` | T08 | same | Fourteen skills |
| `docs/reference/api.md` | T08 | same | rustdoc for both crates, `just doc`, `just site`, Pages |
| `docs/operations/maintenance.md` | T08 | same | L's routine |
| `docs/operations/troubleshooting.md` | T08 | same | L's, minus wasm |
| `docs/decisions/README.md` and fourteen records | T09 | §8 | |

## 7. Agent contract

`AGENTS.md` is canonical; `CLAUDE.md` is exactly `@AGENTS.md`;
`.github/copilot-instructions.md` is B's adapter text. Both adapters and
`.claude/settings.json` are byte-identical to L's. `AGENTS.md` keeps L's section order
with these invariants in place of L's:

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

L's third invariant (errors are thiserror enums) waits for the first error type; L's
fifth (wasm) is dropped. The Provenance section keeps L's sentence containing `runes`.

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
| `plan-change`, `review-change`, `review-docs`, `fix-quality`, `project-check` | same | Gate names and numbers of §4; no wasm |
| `allium`, `distill`, `elicit`, `propagate`, `tend`, `weed`, `witness` | same | Byte-for-byte from L except recipe names; `skills-lock.json` copied verbatim as provenance |

Fourteen skills, twenty-eight bridges. `just check-agents` prints
`Validated AGENTS.md, 2 adapters, and 14 skills.`

## 8. Decision records

A fresh series. A carried record opens "Carried from libpawdoku's decision NNNN at
`ccdd001`" and keeps L's `canonical_for` slug. Format as L's: frontmatter,
`# Decision NNNN: title`, Context, Decision, Consequences, What would reopen this,
Related pages. T09 writes 0001 to 0014; each wave-2 ticket writes its own.

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
| 0010 | `0010-skill-reference-material.md` | Skill reference material beside the skills | 0010 | `decision_skill_reference` |
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
own section; the other rebases and adds its section.

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
behaviour, recorded in T00's hand-back notes and in the module's own comment.

### The effects boundary (T10)

What is banned, in libpachinko only:

- **Crates**, through its normal dependency tree: `rand`, `rand_core`, `getrandom`,
  `tokio`, `async-std`, `smol`, `mio` and `socket2`, and any crate T10 adds with a
  reason. Development dependencies are exempt, so a test may use a seeded generator.
- **std APIs**: `std::time::SystemTime::now`, `std::time::Instant::now`,
  `std::time::Instant::elapsed`, `std::time::SystemTime::elapsed`, everything under
  `std::fs`, `std::env`, `std::net` and `std::process`, `std::thread::spawn`,
  `std::thread::scope`, `std::thread::sleep` and `std::thread::Builder`. Standard I/O
  (`std::io::stdin` and the rest) is not banned: decision 3 does not name it.

The mechanism is T10's choice among these, decided by evidence (§12 claim 2):

- **Crates:** a `cargo tree -p libpachinko -e normal --prefix none --locked --offline`
  line in `scripts/check-effects.sh` that must match none of the banned names (covers
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

T00's version of the script does step 1 for crates only (the `cargo tree` line over an
empty dependency tree) and prints that the std-API bans and the probe arrive with T10, so
gate 8 is green and honest from the first commit.

## 10. CI

L `.github/workflows/ci.yml`, `audit.yml`, `pages.yml` and `.github/actions/setup/action.yml`
at `ccdd001`, with these changes:

- The `wasm` job is deleted, and leaves `check`'s `needs`. The `rust` job, which already
  runs `metrics` and `snapshots-check`, gains `just effects` after `just metrics`.
- `pages.yml` publishes `just site`, which now links both crates.
- The setup action installs every tool except allium; L's description says "The allium
  binary is the documents job's own step". Any job that runs `just check`, `lint`,
  `check-specs` or `analyse-specs` runs `just install-allium` after the setup action:
  L's `documents` job, T00's one-job stub and T14's `release.yml`.
- Every remote `uses:` keeps a full SHA with a version comment, re-read on the day T04
  runs (`gh api repos/<owner>/<repo>/git/ref/tags/<tag>`, dereferencing annotated tags).
  The local `uses: ./.github/actions/setup` has no pin.

The required check is `check`, the aggregate job, from T01 onward and never anything
else. A new gate job joins `check`'s `needs` and nothing else moves. In wave 2, T13's
`image` job joins `needs`, because a Dockerfile that no longer builds is a broken service;
T14's `semver` job (a report on pull requests, as L's draft argues), the scheduled
`mutants.yml` and `fuzz.yml`, and the tag-triggered `release.yml` never do.
`audit` is never in `needs`.

## 11. Rules for tickets and lanes

- **Worktrees and branches.** Each ticket runs on the branch its `branch:` field names
  (`ticket/<id>-<slug>`, lowercase) in its own worktree, created from `main` after every
  ticket it depends on has merged. From a Supacode terminal:
  `supacode repo worktree-new --branch <branch> --base main --name <id>`; otherwise
  `git worktree add ../<id> -b <branch> main`. The tools live in the worktree's own
  gitignored `.pixi/` and `.tools/`, so every new worktree runs `just initialize` (network,
  authorised); only the primary checkout's run installs the shared hook shim, once,
  after T00 merges (`README.md`, "How to pick up a ticket").
- **T00 ships a shape-complete skeleton.** `bg-validate-docs` and `bg-validate-agents`
  can only be green if every page, record and skill exists from the first commit, so T00
  ships a stub for each. Lanes **replace** stubs; they never add, rename or reclassify a
  registered page, record or skill. A lane may create only the paths §3 marks `new` for it.
- **Files no wave-1 lane touches:** `Justfile`, `.pre-commit-config.yaml`,
  `.pre-commit-fix.yaml`, `rust-toolchain.toml`, `tools.txt`, `tools-source.txt`,
  `pixi.lock`, the pins and recipe list in `pyproject.toml`, `docs/manifest.yml`,
  `docs/README.md`, `CLAUDE.md`, `.github/copilot-instructions.md`,
  `.claude/settings.json`, `crates/pachinko/src/main.rs` and every bridge. A lane that
  needs a change there stops and hands it back as a T00 follow-up pull request on `main`.
- **No dependency is added in wave 1.** Nothing uses one yet (§1 fact 6).
- **No path appears in two wave-1 lanes' Files-touched lists.** An agent that finds a
  need to edit another lane's file hands it back instead.
- **Wave 2 shares files by rebasing.** The frozen list lifts after T11. A wave-2 ticket
  may then edit its own lines in any file, frozen or owned, and lists each such file in
  its Files touched as `shared, own lines`. The ones most of them touch are `Justfile`,
  `pyproject.toml`, `pixi.lock`, `tools.txt`, `.gitignore`, `docs/manifest.yml`,
  `docs/README.md` (a new page or record must be registered and reachable),
  `docs/decisions/README.md`, `docs/reference/commands.md` and `CHANGELOG.md`. Wave-2
  tickets merge one at a time, each rebased on the last; `pixi.lock` and `Cargo.lock`
  are regenerated after the rebase, never merged by hand. A recipe a wave-2 ticket adds sits outside `check`'s recipe list; pre-landing
  wave-2 recipes in T00 does not work, because the fuzz crate's `--manifest-path` lines in
  `sync`, `lock`, `fmt-check` and `clippy` would fail before `fuzz/` exists. Each
  wave-2 ticket's first acceptance criterion is its recipe's green output, quoted.
- **Self-contained.** A ticket is written for an agent with no context: it embeds exact
  content or cites this document by section, and names the L, B or T paths to read. It
  never cites a chat transcript, a scratch directory or a tool result.
- **Paths as code spans.** Ticket text writes repository paths and URLs as code spans,
  never as Markdown links: the hook gate runs lychee offline over `tickets/`.
- **Definition of done**, for every build ticket: `just check` green in this repository,
  the ticket's acceptance criteria met, its verification commands run with their output
  quoted in the hand-back notes, the adversarial reviews the ticket names run (§14) with
  every valid finding fixed or handed back, and the ticket's `status:` set to `done` in
  the same pull request. A ticket is done when its gate passes, not when its files exist.
- **Separately authorised actions.** Commits on the ticket branch are the ticket's work.
  Pushing, opening a pull request, tagging, filing issues, changing repository or
  organisation settings, installing a GitHub App, publishing anywhere, installing
  anything outside the worktree, and editing another repository are each separately
  authorised: the ticket says where one occurs, and the agent stops and asks.
- **Network.** `pixi install`, `just install-tools` (and a rustqual source build),
  `just install-allium`, `just install-hooks`, `cargo fetch` and `rustup toolchain install`
  need the network. Each is a named, authorised step in the ticket that runs it.
  `just check` must not need the network.
- **Scratch.** Anything that is not a deliverable goes in `ai_tmp/` (gitignored from T00)
  or the session's scratch directory, never in a commit.
- **Credentials.** No file carries a token. CI uses the run's own `github.token`.

## 12. Unverified claims, and the ticket that checks each

Each claim was reasoned from source or documentation but not executed here. The named
ticket runs the check and records the outcome in its hand-back notes; a claim that fails
is a design change that comes back through this document. T11 carries the outcomes of
the claims wave 0 and wave 1 own (1 to 6, 10 to 12) into this table; T17, the only
ticket after T13, T14 and T16, carries claims 7 to 9 from their hand-back notes. L's own
§12 claims about pixi, prek, binstall and the hooks were settled in L; a ticket re-checks
them only by running the gate.

| # | Claim | Check | Owner |
| --- | --- | --- | --- |
| 1 | allium 3.6.1 reports empty `diagnostics` and `findings` for a module with no declarations | `just check-specs`, `just analyse-specs` on §9's module | T00 first, T06 |
| 2 | Clippy reads `crates/libpachinko/clippy.toml` for libpachinko alone and shadows rather than merges the root file; rustqual pattern rules can be scoped to one crate by running it over that directory | A deliberate violation in a scratch copy, each mechanism in turn | T10 |
| 3 | L's lint table is green on the new stable | `just clippy` on T00's pin | T00, T02 |
| 4 | An integration test running `CARGO_BIN_EXE_pachinko` makes `main.rs` count as covered under `cargo llvm-cov nextest`, so the one-line-main shape reaches 90% | `just coverage`, then `cargo llvm-cov report --show-missing-lines` | T00 first, T02 |
| 5 | B's pixi git dependency installs in `BiscuitBin` CI with no credential | T00's stub run on `main`, read by T01; then T04's full run | T01 first, T04 |
| 6 | T `scripts/bootstrap_repo.sh` works on an organisation-owned repository; Pages works on the free organisation plan for a public repository | The script without `--apply`, then with it | T01 |
| 7 | cargo-semver-checks has a usable baseline before the first release (`--baseline-rev`) | A scratch run against `main` | T14 |
| 8 | cargo-fuzz still works on the pinned stable with `--sanitizer none` | `just fuzz` on the smoke target | T16 |
| 9 | docker is available locally and on `ubuntu-latest` runners | `docker version`; the first CI run | T13 |
| 10 | `just snapshots-check` is green on a workspace with no `insta` dependency and no `.snap` file | The recipe on T00's tree | T00 first, T02 |
| 11 | rustqual 1.8.3 runs over a crate with no `[[architecture.pattern]]` table and two tiny files without a finding | `just metrics` | T00 first, T10 |
| 12 | `cargo tree -p libpachinko -e normal --locked --offline` runs offline after `just sync`, so gate 8 does not need the network | The script with the network blocked | T00 first, T10 |

## 13. Risks every ticket states where it applies

- **The toolchain pin is stale by the time a ticket runs.** A ticket that bumps it bumps
  `rust-version` with it and re-runs `just clippy`, because the lint set moves.
- **pixi's rust shadows rustup** in a shell that does not source `~/.cargo/env`;
  `check-toolchain` is the alarm (L `tickets/CONVENTIONS.md` §13).
- **The small denominator.** Two crates with three items: one untested branch breaches
  90%. Keep each item trivially and fully tested until the specifications bring code.
- **`--locked` and the snapshot.** Any recipe that rewrites `Cargo.lock` or `pixi.lock`
  fails `check-clean`; `target/`, `.tools/` and `.pixi/` are ignored from T00.
- **The network at first run, none in `check`.** As §11.
- **A gate that cannot fire.** A ban with no fixture breaking it proves nothing; gate 8's
  probe is what makes decision 3 mechanical. A lane that weakens the probe hands it back.
- **A red stub run on `main`.** T00's merge is the first time the setup action runs on a
  `BiscuitBin` runner (§12 claim 5). If that run is red and T01 then requires `check`,
  every other lane is blocked until T04 lands. T01 stops instead of requiring a red check.
- **Organisation settings.** `BiscuitBin` is an organisation, not a user; branch
  protection, Pages and GitHub App installation may need an organisation owner. T01 and
  T17 say so before acting.
- **Frozen files and follow-ups.** T02, T06 and T10 are the lanes most likely to need a
  recipe changed; each change is a small `main` pull request, and T11 reconciles what
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
    acceptance criterion asserted but not shown.

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
  and the re-run output. The raw files stay in `ai_tmp/` and are never committed.
