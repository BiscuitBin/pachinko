# Tickets for a project-managed Apalache

Quint's model checker is Apalache. `just quint verify` and `just quint compile --target
tlaplus` run it. Today both fail in this repository, because Apalache runs on a Java
runtime and the pixi environment has none (`/usr/bin/java` on macOS is a stub that
offers to install one). Decision 0021, `docs/decisions/0021-project-managed-quint-cli.md`,
left Java out on purpose and left Apalache's download unchecked.

These two tickets bring Apalache under the project the way every other tool is held:
pinned, checked against a checksum this repository records, installed into the worktree
by `just initialize`, and never written to the home directory or the global `PATH`. Each
cites `tickets/CONVENTIONS.md` by section and follows the format of `tickets/`.

## Before AP01 starts

- **The Quint CLI must be in the base.** Both tickets build on `tools/quint/`, the
  `just quint` recipe and `QUINT_HOME`, which land with the pull request that pins the
  Quint CLI 0.34.0 (branch `quint-spec`). If that pull request has merged, AP01 bases on
  `main`. If it has not, AP01 bases on `quint-spec`, and the maintainer says so before
  step 1. The base branch and the worktree are the maintainer's choice.
- **AP02 bases on a branch that holds AP01.** Its design (a pin file, a stamp and a
  gate that checks the pin was reviewed) is a proposal the maintainer rules on before
  AP02 step 4.
- **These files are committed on `quint-spec`.** A new worktree based on a branch that
  holds that commit sees them.

## Index

The `status:` field in each ticket's frontmatter is authoritative; this table is a
snapshot.

| Id | Title | File | Blocked by | Status |
| --- | --- | --- | --- | --- |
| AP01 | A pinned Java runtime, and a working `just quint verify` | `AP01-java-and-verify.md` | the Quint CLI pin | done |
| AP02 | Apalache pinned by checksum, installed by `just initialize` | `AP02-checksummed-apalache.md` | AP01 | todo |

## Dependencies

```text
Quint CLI pin (quint-spec) ──> AP01 ──> AP02
```

AP01 makes Apalache run: it pins conda-forge's `openjdk` in pixi and proves that
`just quint verify` works inside the worktree, with Quint's own download of Apalache.
AP02 then replaces that download, wherever `just install-apalache` has run, with one this
repository checks. Its gate fails on any Quint upgrade until someone has reviewed the
Apalache pin against the new CLI and recorded that review in the pin file. The gate
cannot read the CLI's Apalache version in CI, where the CLI is not installed. The
installer reads it, and refuses a pin that disagrees. Quint still downloads Apalache
unchecked if `.quint/` is missing or a run passes `--apalache-version`. AP02 records that
gap and the guard that would close it.

## Out of scope for both

- The Rust evaluator Quint downloads for `run`, `test` and the REPL. It has the same
  unchecked download and could be pinned the same way in a later ticket.
- A gate or a CI job that runs `quint verify`. There is no `.qnt` module in the
  repository yet, and a gate over zero modules asserts nothing.
- Changing how Quint finds an Apalache server. Quint uses any server already listening on
  `localhost:8822` before it starts its own. Both tickets document this, and their
  verification checks that the port is free, but neither changes it.

## Adversarial review

Each ticket runs `tickets/CONVENTIONS.md` §14 twice, with Codex (Astra, `gpt-6-astra`) as
the reviewer. The first checkpoint reviews the builder's plan before any file changes.
The second reviews the diff before the final commit. T37, which corrects §14's command,
is still todo, so AP01 embeds the corrected command and AP02 cites it.
