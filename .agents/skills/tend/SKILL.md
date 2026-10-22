---
name: tend
description: "Tend the Allium garden. Use when the user wants to write, edit, update, add to, improve, clarify, refine, restructure, fix or migrate Allium specs. Covers adding entities, rules, triggers, surfaces and contracts, fixing syntax or validation errors, renaming or refactoring within specs, migrating specs to a new language version, and translating requirements into well-formed specifications. Pushes back on vague requirements."
---

This skill is a stub that ticket T05 replaces. Until then, follow `AGENTS.md`, which is canonical and wins wherever the two disagree.

When T05 writes it, this file will hold the upstream Allium `tend` skill, as the frontmatter above describes it. Once written, it will match the copy the upstream library carries byte for byte, except where a recipe name differs; its reference material will live under `allium-skill-reference/` at the repository root, which also arrives with T05.

The recipes it will name include `just check-specs` and `just analyse-specs`. Run the narrowest of them while working, then `just check` before handing the work back.
