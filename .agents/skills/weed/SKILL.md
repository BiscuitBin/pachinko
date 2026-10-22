---
name: weed
description: "Weed the Allium garden. Find where Allium specifications and implementation code have diverged, and help resolve the divergences. Use when the user wants to check spec-code alignment, compare specs against implementation, audit for spec drift or violations, sync specs with code or code with specs, or verify whether the implementation matches what the spec says."
---

This skill is a stub that ticket T05 replaces. Until then, follow `AGENTS.md`, which is canonical and wins wherever the two disagree.

When T05 writes it, this file will hold the upstream Allium `weed` skill, as the frontmatter above describes it. Once written, it will match the copy the upstream library carries byte for byte, except where a recipe name differs; its reference material will live under `allium-skill-reference/` at the repository root, which also arrives with T05.

The recipes it will name include `just check-specs` and `just analyse-specs`. Run the narrowest of them while working, then `just check` before handing the work back.
