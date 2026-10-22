---
name: propagate
description: "Generate tests from Allium specifications. Use when the user wants to propagate tests, generate test files from a spec, write tests for a specification, create property-based tests, produce state machine tests, check test coverage against spec obligations, or understand what tests a specification requires."
---

This skill is a stub that ticket T05 replaces. Until then, follow `AGENTS.md`, which is canonical and wins wherever the two disagree.

When T05 writes it, this file will hold the upstream Allium `propagate` skill, as the frontmatter above describes it. Once written, it will match the copy the upstream library carries byte for byte, except where a recipe name differs; its reference material will live under `allium-skill-reference/` at the repository root, which also arrives with T05.

The recipes it will name include `just check-specs` and `just analyse-specs`. Run the narrowest of them while working, then `just check` before handing the work back.
