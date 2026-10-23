---
title: "Decision 0012: The metrics gate"
kind: "decision"
audience: [contributor, maintainer, agent]
canonical_for: [decision_metrics_gate]
requires: []
---

# Decision 0012: The metrics gate

*Carried from libpawdoku's decision 0013 at `ccdd001`.*

## Context

Without this gate the code's shape would be measured in one place. Clippy runs
`too_many_lines`, from its `pedantic` group, and `too_many_arguments`, from its
`complexity` group, and at clippy's defaults of 100 lines and 7 arguments they shape
little; nothing else looks at size, cohesion or coupling.

The maintainer of the source library asked for four families of metric as a required
gate: complexity, cohesion, coupling and module boundaries. They asked for them as
guardrails set before the code they guard exists, so that the numbers shape the code
rather than being fitted to it. A spike there weighed eleven tools and recommended two
things: thresholds for the lints clippy already runs, and rustqual as a new recipe. The
maintainer then settled its open points: a required gate with a stated exit, a second pin
list for a tool that must be compiled, and a nesting limit of 4 in both tools.

This repository is in the same position, more so: its domain is not defined yet, and the
specifications under `docs/specs/` will define it. The gate was carried with the rest of
the gate list at the foundation, so the numbers are in place before the first line of
domain code. [Decision 0009](0009-rust-quality-gate.md) records the gate's other Rust
recipes; this record holds the one that measures shape.

## Decision

**Clippy's thresholds.** `clippy.toml` sets four, and the workspace lint table names
`cognitive_complexity`, a restriction lint the `pedantic` group does not enable.
`excessive_nesting` is already live through the `complexity` group but silent until its
threshold is set.

**A `metrics` recipe at gate 7**, between `clippy` and `effects`, in `just check` and in
the `rust` CI job. It runs rustqual against the committed `rustqual.toml` with
`--fail-on-warnings`, once per crate: over `crates/libpachinko` and over
`crates/pachinko`. rustqual's dimensions are on for complexity, SRP (cohesion and size),
coupling and architecture; duplicates, boilerplate, test quality and the structural
checks are off, as is the magic-number check, because each is the author's opinion rather
than a metric the maintainer asked for.

**No module-boundary rules yet.** The source record's architecture dimension held one
`[[architecture.pattern]]` rule per module of the source library's module layering, each
forbidding every `crate::` path its module may not name, and a probe over a fixture that
broke each rule once, because rustqual reports a misconfigured architecture section as a
clean one. Neither is carried. Each crate here is one module, `lib.rs`, so there is
nothing to bound and no rule to probe. The architecture dimension stays on, and
[Layering](../explanation/layering.md) sets out the rules it will hold: they return with
the first specification that names two modules, and their probe returns with them, for
the reason the source record gave.

**The effect bans are not this gate.** They are boundary rules of a kind, but they belong
to gate 8, `just effects`, with their own probe; [decision 0008](0008-std-library.md)
records why. `rustqual.toml` runs over `crates/pachinko` too, where every effect is
allowed, so it cannot hold a ban that applies to libpachinko alone.

**The thresholds.** Every key that can fail the gate is written in `rustqual.toml` and
has a row below; none is left to rustqual's default. Most rows keep the default rustqual
documents at 1.8.3, now as a choice, and three depart from it: the file length and the
two limits on test code. They were chosen before the code they guard and not tuned to a
tree; the same numbers apply to both crates.

The four clippy can also check carry the same number in `clippy.toml`, but the two tools
do not count alike. rustqual measures a function from its opening brace to its closing
one, comments and blank lines included, where clippy's `too_many_lines` skips both; so
rustqual fires first on a commented function. rustqual also keeps no metrics for a
function with no logic and no call to the crate's own functions, so it never checks
that function's length, and there clippy is the only guard.

| Guardrail | Value | Why this number |
| --- | --- | --- |
| Cognitive complexity per function | 15 | The default of the metric's originator, SonarSource, and rustqual's; clippy's is 25. |
| Cyclomatic complexity per function | 10 | McCabe's own recommended ceiling for the measure, and rustqual's default. |
| Nesting depth | 4 | rustqual's default, and the limit both tools hold. |
| Function length | 60 lines | rustqual's default: a function that fits on one screen. Clippy's 100 is too loose to shape anything. |
| Test function length | 100 lines | Looser than production on purpose: a table of cases is long without being complex. rustqual's default would hold tests to 60. |
| Recursion | forbidden | rustqual's default, kept as a choice: a recursive call in a function with logic is an IOSP finding. A search keeps an explicit stack, whose depth is then bounded where it can be read. |
| Closures, iterator chains and `?` | not counted as logic | rustqual's defaults. All three are idiomatic Rust, and IOSP counting them would push the code away from the idiom. |
| Parameters | 5 | rustqual's default. A sixth is usually a struct waiting to be named; clippy's default is 7. |
| LCOM4 per struct | 2 | rustqual's default: one separable cluster of methods is tolerated before cohesion counts against a struct. |
| Fields and methods per struct | 12 and 20 | rustqual's defaults, the sizes over which the SRP score weighs a struct more heavily. |
| Fan-out per struct | 10 | rustqual's default: the count of other types a struct's methods reach, an input to the SRP score. It is not the per-module fan-out below. |
| SRP score per struct | 0.6 | rustqual's default, with its default weights of 0.4 for LCOM4, 0.25 for fields, 0.15 for methods and 0.2 for fan-out. A struct at or over it is reported. |
| File length | 500 code lines | Counted before the first `#[cfg(test)]`, without comments, doc comments or blank lines. Rust has no norm here and clippy no lint; a Rust module keeps a type beside its impls, so rustqual's 300 would force splits that serve the tool. |
| Test file length | 1,000 code lines | Applies to files under `tests/` and to test-only modules. Looser for the reason the test function limit is. |
| Independent clusters per file | 2 | rustqual's default: a file is reported at three groups of private functions, of five statements or more each, that never call one another. |
| Instability of a module something imports | 0.8 | rustqual's default. A module nothing imports is exempt; the cap guards the modules others lean on. |
| Fan-in and fan-out per module | 15 and 12 | rustqual's defaults. With one module per crate today, they bind only once the crates have modules. |
| Stable Dependencies Principle | on | rustqual's default: a module may not depend on one less stable than itself. |
| Module boundaries | none yet | No module to bound; the rules return with the first specification that names two modules. |
| Suppression budget | 5% of functions | rustqual's default (`max_suppression_ratio`). Small enough that a suppression stays an exception. |
| Headroom of a suppression pin | 10% | rustqual's default (`pin_headroom`). A suppression that names a limit, as `file_length=600` does, is itself a finding when the limit sits more than 10% above the value it covers, so a pin cannot absorb later growth. |

Two of rustqual's checks are switched off because another gate owns the rule. Its check
on `unsafe` belongs to `unsafe_code = "forbid"`, and its check on `unwrap`, `expect` and
`panic!` belongs to clippy's `unwrap_used`, `expect_used` and `panic`. Left on, a
justified `#[expect]` on one of those lints would need a second suppression in rustqual's
syntax, charged to the budget above.

**`tools-source.txt`, the one exception to the binary-only rule.** rustqual publishes no
release binary, for any of its versions, and is on neither conda-forge nor Homebrew. So
`tools.txt` stays binary-only, and a second list, `tools-source.txt`, holds
`rustqual@1.8.3`. `just install-tools` reads it in a second pass that allows
cargo-binstall's compile strategy; cargo-binstall passes `--locked` through to
`cargo install`, so the build takes the versions rustqual's own lockfile names.
cargo-binstall tries a signed cargo-quickinstall build first and would take one that
appeared for the pin; `--only-signed` makes the signature a requirement rather than a
check made when one exists, and still leaves the source build. The pass disables the
`crate-meta-data` strategy, which cargo-binstall would otherwise try before either: it
takes a release asset from the crate's own repository, which rustqual would publish
unsigned, so a release appearing upstream cannot bypass both the signature and the
registry checksum. The pass also runs without a GitHub token, so no build script it
compiles can read one. Compiling is acceptable for this one tool because it is paid once
per worktree and, in CI, once per pin by each job that runs the setup action on a cold
cache: the four `ci.yml` jobs in parallel, `audit.yml`'s `audit` job and `pages.yml`'s
`build` job; `.tools` is then cached on the hash of both lists. A line
joins this list only on a decision record that accepts the same cost.

**The pin** is held at 1.8.3 and bumped by hand, one release at a time, with `just check`
run on each. Nothing bumps a tool pin automatically today. `just metrics` fails unless the
rustqual it finds reports the pinned version, so a missing `.tools/bin/rustqual` cannot
fall through to another on `PATH`.

## Consequences

The ones that hurt.

- **A young tool by one author.** rustqual was five months old at 1.8.3, with eight minor
  versions since 1.0. A bump may rename a rule, change a default or tighten a heuristic,
  and each bump is then a gate failure to triage rather than a report to read. The pin
  defers that cost; it does not remove it.
- **IOSP cannot be switched off.** It is the author's own dimension and it has no
  `enabled` key. If the domain code's natural shape offends it, the gate fails on an
  opinion the project never adopted. One case is known and chosen: recursion is a
  finding, so a search keeps an explicit stack. An IOSP suppression is one line with a
  reason, `// qual:allow(iosp) reason: "..."`, which is the form `AGENTS.md` permits: one
  rule on one line with a stated reason. But the suppression budget is 5% of the function
  count, and it charges `#[allow]` and `qual:allow` lines while `#[expect]` goes free.
- **A source build at bootstrap.** `just initialize` runs `just install-tools`, which
  compiles a Rust program and so needs a toolchain and the network. On a cold CI cache
  every job that runs the setup action builds it: the four `ci.yml` jobs in parallel,
  `audit.yml`'s `audit` job and `pages.yml`'s `build` job.
- **A green run can hide a dead section.** With no boundary rules there is no probe, and
  nothing proves the complexity, SRP and coupling sections are running with the numbers
  written; a green gate would look the same if one had quietly stopped measuring.
- **The module direction has no mechanical guard yet.** rustc allows a cycle between
  modules of one crate, and until the boundary rules return, review alone would hold the
  direction. When they return they bring the blind spots
  [Layering](../explanation/layering.md) lists, and what guards each.

## What would reopen this

**The exit.** If two consecutive bumps of the rustqual pin each need a suppression or a
threshold change, `metrics` leaves `just check` for an advisory job on the pattern of
`audit.yml`, and any boundary rules then in place are kept as a test through archaven, a
development dependency.

**The return of the boundary rules.** The first specification that names two modules
adds one `[[architecture.pattern]]` rule per module to `rustqual.toml` and a fixture that
breaks each rule once, which `just metrics` must see fail. That change amends this
record.

ast-metrics is the other alternative, installed by a checksummed download as the Allium
checker is: it ships release binaries, and it would replace rustqual for complexity and
coupling if rustqual were dropped.

An upstream `enabled` key for IOSP would remove the second consequence above. rustqual
publishing release binaries, or arriving on conda-forge, would move its pin into
`tools.txt` or `pyproject.toml` and empty `tools-source.txt`.

## Related pages

- [Decision 0008: A std library, with effects kept out mechanically](0008-std-library.md)
- [Decision 0009: The Rust quality gate](0009-rust-quality-gate.md)
- [Quality gates](../reference/quality-gates.md)
- [Layering and dependency direction](../explanation/layering.md)
- [Maintain dependencies](../how-to/maintain-dependencies.md)
