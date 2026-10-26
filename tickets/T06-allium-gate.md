---
id: T06
title: "Allium gate: the placeholder module, the spec recipes, the two spec pages"
status: done
depends_on: [T00]
parallel_with: [T01, T02, T05, T07, T10]
branch: ticket/t06-allium-gate
estimated_size: S
---

# T06: Allium gate: the placeholder module, the spec recipes, the two spec pages

## Context

The specifications come after this foundation, but the gate that checks them comes now,
so that the first real module lands into a working loop. B `src/biscuit_games_tooling/run_allium.py`
line 131 refuses an empty `docs/specs/`, so T00 shipped `docs/specs/pachinko.allium`, a
root module that declares no behaviour (`CONVENTIONS.md` §1 fact 1, §9). This lane makes
it final, proves every Allium recipe on it, and writes the two pages that explain the
specification workflow.

L `tickets/T06-spec-migration.md` is the nearest procedure, though it migrated seven real
modules; its hand-back notes record allium 3.6.1's behaviour. L
`docs/how-to/work-with-the-specs.md` and L `docs/explanation/specifications.md` are the
pages this lane adapts. L `tickets/T24-plan-spec-recipe.md` is the `plan-spec` recipe's
origin.

## Goal

`just install-allium`, `just check-specs`, `just analyse-specs` and `just plan-spec
pachinko` are each green on the placeholder, which is final and says plainly that it
holds no behaviour yet; the zero-diagnostics, zero-findings result is quoted; and the two
spec pages describe the workflow the first real module will follow.

## Non-goals

- No behaviour in the module: no entity, rule, surface, invariant or config that a test
  would have to honour.
- No second module.
- No recipe change; a flag found wrong is a T00 follow-up.
- No handbook page beyond the two this lane owns.

## Files touched

| Path | Class | Content |
| --- | --- | --- |
| `docs/specs/pachinko.allium` | final | `CONVENTIONS.md` §9 |
| `docs/how-to/work-with-the-specs.md` | final | L's, the module table with one row, the waiver terms verbatim |
| `docs/explanation/specifications.md` | final | L's argument; one module that states nothing yet |
| `tickets/T06-allium-gate.md` | ticket | `status:` and hand-back notes |

## Steps

1. Create the worktree after T00 has merged.
2. **The module.** Read T00's hand-back notes for the outcome of `CONVENTIONS.md` §12
   claim 1. Finalise the header comments: Scope (the domain is undefined; this is its
   root), Includes (nothing yet), Excludes (every behaviour until a specification change
   defines one), Dependencies (none), Vocabulary (none yet; `docs/project/terminology.md`
   is the glossary). Keep `-- allium: 3`.
3. **The recipes.** `just install-allium` (network, authorised), confirm the binary is
   3.6.1 with B's checksum, then `just check-specs`, `just analyse-specs` and
   `just plan-spec pachinko`. Quote each.
4. **The gate fires.** In a scratch commit, add a malformed line to the module and confirm
   `just check-specs` fails; add a rule with an unreachable condition and confirm
   `just analyse-specs` reports a finding. Revert both. This shows the empty-result green
   is not a silent pass.
5. **`work-with-the-specs.md`.** L's page, with the module table reduced to the one
   placeholder, the waiver terms verbatim (L's "Diagnostics and waivers" section), and a
   short section on adding the first module: where it goes, how the root imports it, and
   which skill drives it (`spec-change`, then `propagate`).
6. **`specifications.md`.** L's argument that the specifications decide behaviour; the
   state today (one module, no behaviour) and what changes when the first lands. Cite
   decision 0002.
7. `just check-docs`, `just check`, the final adversarial review, `just check` again if
   fixes landed, then commit.

   **Adversarial review** (`CONVENTIONS.md` §14): the lane's diff and step 4's quoted
   failures. Ask: whether the empty-result green could still be a silent pass that step 4
   does not rule out; whether the waiver terms in `work-with-the-specs.md` are L's,
   verbatim.

## Acceptance criteria

- `just check-specs` and `just analyse-specs` each print one block with empty
  `diagnostics` and `findings`.
- `just plan-spec pachinko` exits 0; its output is quoted.
- Step 4's two failures are quoted.
- `just check` is green.
- The adversarial reviews in Steps ran (`CONVENTIONS.md` §14), and every valid finding is
  fixed or handed back.

## Verification

```sh
.tools/bin/allium --version
just check-specs
just analyse-specs
just plan-spec pachinko
just check-docs
just check
```

## Hand-back notes

### Baseline: `bf51e0d`

The untouched worktree was clean. The initial shell selected Pixi's cargo; rerunning
`just check-toolchain` with rustup first on PATH fixed the environment. The sandbox
then blocked prek's cache log; the authorized retry of `just lint` passed.
Subsequent full gates use `PATH="$HOME/.cargo/bin:$PATH"` and existing cache access.

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

Created `ticket/t06-allium-gate` from the supplied `ticket/06-allium-gate` branch
at the same commit, in the existing dedicated worktree. No initialization or hook
installation was needed.

### Installation and placeholder: `bf51e0d` with uncommitted changes

```text
just install-allium
bg-install-allium
allium 3.6.1 is already installed at /Users/scutting/.supacode/repos/pachinko/ticket/06-allium-gate/.tools/bin/allium
Exit status: 0

.tools/bin/allium --version
allium 3.6.1 (language versions: 1, 2, 3)
Exit status: 0
```

The installer skips downloading when the version matches. Independently downloaded
the release into ignored scratch space and verified its archive SHA256 against B
`src/biscuit_games_tooling/install_allium.py` at `6c5c07f`. The installed binary is
byte-identical to the executable inside that checksummed archive.

```text
aarch64-apple-darwin archive SHA256:
ecfae02fcf8e60475a014944183158ccde4104f6f0223e1bf91f98519fcb19eb
Installed binary matches checksummed release: True
Binary SHA256:
f41d5cb75cee110d6cb1b92f6d146ae84eacc5929aa28f65380e8f072152b2fd
Exit status: 0
```

CONVENTIONS §12 claim 1 holds, as T00 reported: a comments-only version-3 module
needs no fallback declaration. The final module adds no behaviour or open question.

```text
just check-specs
{
  "command": "check",
  "diagnostics": [],
  "findings": [],
  "spec_file": "docs/specs/pachinko.allium"
}
allium check: 1 specifications, no diagnostics and no findings.
Exit status: 0

just analyse-specs
{
  "command": "analyse",
  "diagnostics": [],
  "findings": [],
  "spec_file": "docs/specs/pachinko.allium"
}
allium analyse: 1 specifications, no diagnostics and no findings.
Exit status: 0

just plan-spec pachinko
{
  "diagnostics": [],
  "obligations": [],
  "version": 3
}
Exit status: 0
```

### Negative controls: isolated scratch commits

Scratch commits used separate Git indexes and `git commit-tree`; neither became a
parent of the ticket branch. Each probe replaced only the module for its recipe
run, and the final placeholder was restored afterward. Intentionally failing specs
did not pass through the normal commit hook. Handbook edits were uncommitted during
the runs and do not affect these recipes.

#### Malformed module: `5114d699180a827bfd7a6350dccfbd6cab2ca447`

Appended `entity {` to the final header. Output excerpt:

```text
just check-specs
docs/specs/pachinko.allium: 1 diagnostic
  docs/specs/pachinko.allium:23:8: error: parse error: expected entity name, found '{'
allium check reported 1 diagnostic; the specifications must report none.
error: recipe `check-specs` failed on line 244 with exit code 1
Exit status: 1
```

The JSON contained an error diagnostic with null code and empty findings.

#### Unreachable guard: `72580f9d15fb7c138d4e9aec86686e7beae295a7`

Appended this isolated fixture; none remains in the final module:

```allium
entity Probe {
    status: pending | done
    ready: Boolean
    transitions status {
        pending -> done
        terminal: done
    }
}
given {
    probe: Probe
}
rule Start {
    when: StartProbe()
    ensures: Probe.created(status: pending)
}
rule NeverRuns {
    when: probe: Probe.status becomes pending
    requires:
        probe.status = pending
        probe.ready = true
    ensures: probe.status = done
}
surface ProbeSurface {
    facing probe: Probe
    exposes:
        probe.status
    provides:
        StartProbe()
}
```

```text
just check-specs
allium check: 1 specifications, no diagnostics and no findings.
Exit status: 0

just analyse-specs
docs/specs/pachinko.allium: 3 findings
  dead_transition: Transition 'pending' → 'done' on entity 'Probe' is declared but unachievable [Probe]
  deadlock: Entity 'Probe' can reach state 'pending' but has no achievable path to any terminal state [Probe]
  missing_producer: Nothing establishes Probe.ready = true [Probe, rule NeverRuns]
allium analyse reported 3 findings; the specifications must report none.
error: recipe `analyse-specs` failed on line 247 with exit code 1
Exit status: 1
```

The analysis JSON had empty diagnostics and exactly those three findings. Thus
diagnostics alone cannot explain the failed analysis recipe.

At `bf51e0d` with the fixture temporarily repaired to create `ready: true`:

```text
just analyse-specs
allium analyse: 1 specifications, no diagnostics and no findings.
Exit status: 0
```

Earlier exploratory fixtures were insufficient: contradictory numeric guards were
not detected, and assigning the guarded field at creation counted as a producer
even when its value differed. The final control tests a missing producer, not
general logical satisfiability. Reading Allium 3.6.1 source in scratch space
confirmed that `crates/allium-parser/src/analysis.rs` checks whether a field has a
producer. Green analysis is not proof that every condition can be satisfied.

### Documentation and review

The entire "Diagnostics and waivers" section is copied byte for byte from L
`docs/how-to/work-with-the-specs.md` at `ccdd001`. A preceding paragraph identifies
its historical examples. Both pages keep their registered frontmatter and links.

At `bf51e0d` with uncommitted changes:

```text
just check-docs
markdownlint.............................................................Passed
typos....................................................................Passed
lychee...................................................................Passed
bg-validate-docs
Validated 39 pages and 40 canonical topics.
Exit status: 0
```

At `bf51e0d` with uncommitted changes, the pre-review full gate and test suite passed:

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0

just test
3 tests run: 3 passed, 0 skipped
libpachinko doctests: 1 passed; 0 failed
pachinko doctests: 0 passed; 0 failed
Exit status: 0
```

The required external Claude review call was rejected by automatic approval review
as an insufficiently authorized export. It was not executed. Used the §14 fallback
of fresh-context Codex reviewers on `gpt-6-astra` within the current session instead.

- Breakage: no findings. The reviewer read pinned B's wrapper, checked both controls
  and their repair, verified the waiver text, and independently reran the spec
  recipes and empty plan successfully.
- Initial conformance: no findings, but the reviewer accidentally read the hand-back
  conclusions. A fresh review excluding those notes replaces it as the checkpoint
  evidence.

- Fresh conformance: no findings. The reviewer remained blind to hand-back notes
  and earlier reviews, checked the four-path scope and raw probe evidence, compared
  the waiver section to L, and independently ran `just plan-spec pachinko`.
- Triage: no findings to validate, fix or hand back. No high-severity findings,
  no second fix round, and no changes to recipes or another lane's files.

The final verification uses `bf51e0d` with the completed uncommitted T06 changes:

```text
just check
All checks passed and the worktree is unchanged.
Exit status: 0
```

Raw review records and probe output stay in ignored scratch space. The final
commit contains only the four ticket-owned files; no probe declarations remain.

## Open points

None at hand-back. The analyser limit recorded under Negative controls, that a green
`just analyse-specs` does not prove a guard is satisfiable, was closed on 2026-10-24 on
branch `review-and-finish-bootstrap` (commit `e00bc3b`):
`docs/how-to/work-with-the-specs.md` now says the analyser looks for a producer, and that
contradictory guards pass.
