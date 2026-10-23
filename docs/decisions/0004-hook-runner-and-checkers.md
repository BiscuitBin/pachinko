---
title: "Decision 0004: Hook runner and checkers"
kind: "decision"
audience: [maintainer, agent]
canonical_for: [decision_python_toolchain]
requires: []
---

# Decision 0004: Hook runner and checkers

*Carried from libpawdoku's decision 0004 at `ccdd001`, itself carried from Pawdoku's decision 0004 at `78d03cdf`.*

## Context

This repository ships no Python. Its quality gate is the one every Biscuit Games
repository runs: `prek` runs the hooks, and six console scripts from the package
`biscuit-games-tooling` do the work no hook does: the documentation contract, the agent
contract, the ordered gate with a worktree snapshot between recipes, the pinned Allium
binary and its JSON-reading runner, and a ripsecrets wrapper that keeps matched secrets
out of logs. The package is a git dependency in `pyproject.toml`, and the recipes and
hooks that need it call its scripts by name (`bg-validate-docs`, `bg-validate-agents`,
`bg-project-check`, `bg-install-allium`, `bg-run-allium`, `bg-ripsecrets`).

A Rust repository could go Python-free. `prek` 0.5.3 is a Rust binary, so nothing about
the hook runner itself needs an interpreter. The gate runner, the Allium installer and
runner, and the ripsecrets wrapper are 631 lines of Python that a POSIX shell could port,
with `jq` for the JSON. The two contract validators are 592 lines with no implementation
in any other language anywhere in the house; a Rust rewrite would be a project of its
own, and it would have to land before the first commit of this repository could pass its
own gate.

One fact is new to a Rust repository. The agent contract's checker requires the literal
word `runes` in `AGENTS.md`, because the games are Svelte and the phrase names their
reactivity rule; a Rust workspace has no such rule.

## Decision

Keep the Python toolchain. `pyproject.toml` pins `prek` at 0.5.3 and
`biscuit-games-tooling` at the release tag `v0.3.0`; how those pins are resolved, locked
and installed is [decision 0011](0011-tool-manager.md)'s. No `ruff`: no `.py` file ships,
and the shell scripts are checked by shellcheck. `[tool.biscuit-games-tooling]` lists the
nineteen recipes `just check` runs, in order, and declares no predicates. The hook runner
and the Allium pin stay where the package puts them ([decision
0005](0005-project-managed-allium-cli.md)).

Until the package ships a release in which the required phrases are configurable,
`AGENTS.md` carries one honest sentence in its Provenance section: "This repository has
no Svelte runes; the games' reactivity rule does not apply here." The pin bump to such a
release removes the sentence.

Decision 0011 supersedes this record in part, as it does in the repository this record
was carried from, where this record also chose the installer for the Python side and 0011
later replaced it with pixi. Here
everything about installing, locking and caching the two pins is 0011's. What stands in
this record is the hook runner, the checkers at a pinned tag, `pyproject.toml` as the
place the package reads its configuration, and the interim `runes` sentence.

## Consequences

The gate is the same gate every other house repository runs, from the same release of
the same package. `just check` here means what it means in the games, and a fix to a
checker arrives as a moved pin rather than as an edit merged into a fork. The
documentation and agent contracts are enforced mechanically from the first commit,
which is what lets the foundation ticket ship a shape-complete skeleton and every later
lane replace stubs under a green gate. Neither crate contains any Python.

The cost is one toolchain that a Rust maintainer would not otherwise install, and one
interim sentence in `AGENTS.md` that exists only to satisfy a checker written for a
different stack. Both are accepted deliberately, and the second is temporary. What the
toolchain asks of a contributor and of CI is set out in decision 0011.

## What would reopen this

The checkers being rewritten in Rust inside `biscuit-games-tooling` itself, so that every
consumer could take them as binaries; there is no plan to do that today, so this record
is the steady state and not a stopgap. The two validators becoming simple enough that a
rewrite is cheaper than an interpreter. Either would have to replace the whole package,
because the gate runner, the Allium installer and runner and the ripsecrets wrapper come
from it as well; the runner alone leaving Python is already possible and changes nothing.

## Related pages

- [Decision 0011: Tool manager](0011-tool-manager.md)
- [Decision 0005: A project-managed Allium binary](0005-project-managed-allium-cli.md)
- [Quality gates](../reference/quality-gates.md)
- [Agent contract](../reference/agent-contract.md)
- [Maintain dependencies](../how-to/maintain-dependencies.md)
