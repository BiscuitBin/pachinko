---
name: distill
description: "Extract an Allium specification from an existing codebase. Use when the user has existing code and wants to distil behaviour into a spec, reverse engineer a specification from implementation, generate a spec from code, turn implementation into a behavioural specification, or document what a codebase does in Allium terms."
---

This skill is a stub that ticket T05 replaces. Until then, follow `AGENTS.md`, which is canonical and wins wherever the two disagree.

When T05 writes it, this file will hold the upstream Allium `distill` skill, as the frontmatter above describes it. Once written, it will match the copy the upstream library carries byte for byte, except where a recipe name differs; its reference material will live under `allium-skill-reference/` at the repository root, which also arrives with T05.

The recipes it will name include `just check-specs` and `just analyse-specs`. Run the narrowest of them while working, then `just check` before handing the work back.
